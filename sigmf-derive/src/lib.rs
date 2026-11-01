//! Derive macro for typed [SigMF](https://sigmf.org) metadata extensions.
//!
//! This crate is an implementation detail of [`wcsng-sigmf`]. Enable its
//! `derive` feature and use the macro as `sigmf::SigmfExtension` rather than
//! depending on this crate directly:
//!
//! ```toml
//! [dependencies]
//! wcsng-sigmf = { version = "0.1", features = ["derive"] }
//! ```
//!
//! [`wcsng-sigmf`]: https://docs.rs/wcsng-sigmf
#![warn(missing_docs)]

use proc_macro::TokenStream;
use quote::quote;
use syn::{Attribute, Data, DeriveInput, Fields, LitStr, Path, Result, parse_macro_input};

/// Implements `sigmf::extensions::SigmfExtension` for a struct.
///
/// # Container attributes
///
/// `#[sigmf(...)]` on the struct accepts:
///
/// - `namespace = "..."` (required): the extension namespace. It must be a
///   non-empty ASCII identifier (letters, digits, `_`, `-`) other than `core`.
/// - `version = "..."` (required): the extension version declared in
///   `core:extensions`.
/// - `scope = "..."` (required): where the fields live: `global`, `capture`,
///   `annotation`, or `collection`.
/// - `crate = "..."` (optional): path to the `sigmf` library if it is renamed
///   in `Cargo.toml`; defaults to `::sigmf`.
///
/// # Field requirements
///
/// The struct must have named fields, each with an explicit
/// `#[serde(rename = "<namespace>:<name>")]` that uses the container's
/// namespace. Serde attributes that change the wire shape (`flatten`, `skip*`,
/// `with`, `rename_all`, `tag`, `untagged`, `transparent`, ...) are rejected so
/// that the generated `FIELDS` list always matches the serialized keys.
///
/// # Example
///
/// ```ignore
/// use serde::{Deserialize, Serialize};
/// use sigmf::SigmfExtension;
///
/// #[derive(Serialize, Deserialize, SigmfExtension)]
/// #[sigmf(namespace = "lab", version = "1.0.0", scope = "global")]
/// struct LabReceiver {
///     #[serde(rename = "lab:gain_db")]
///     gain_db: f64,
///     #[serde(rename = "lab:device_id", skip_serializing_if = "Option::is_none")]
///     device_id: Option<String>,
/// }
/// ```
#[proc_macro_derive(SigmfExtension, attributes(sigmf))]
pub fn derive_sigmf_extension(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    expand(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(input: DeriveInput) -> Result<proc_macro2::TokenStream> {
    let mut namespace = None;
    let mut version = None;
    let mut scope = None;
    let mut crate_path: Path = syn::parse_quote!(::sigmf);
    let mut crate_seen = false;

    for attribute in input
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("sigmf"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("namespace") {
                if namespace.is_some() {
                    return Err(meta.error("duplicate sigmf namespace attribute"));
                }
                namespace = Some(meta.value()?.parse::<LitStr>()?);
            } else if meta.path.is_ident("version") {
                if version.is_some() {
                    return Err(meta.error("duplicate sigmf version attribute"));
                }
                version = Some(meta.value()?.parse::<LitStr>()?);
            } else if meta.path.is_ident("scope") {
                if scope.is_some() {
                    return Err(meta.error("duplicate sigmf scope attribute"));
                }
                scope = Some(meta.value()?.parse::<LitStr>()?);
            } else if meta.path.is_ident("crate") {
                if crate_seen {
                    return Err(meta.error("duplicate sigmf crate attribute"));
                }
                crate_seen = true;
                let path = meta.value()?.parse::<LitStr>()?;
                crate_path = path.parse()?;
            } else {
                return Err(meta.error("unsupported sigmf attribute"));
            }
            Ok(())
        })?;
    }

    let namespace = namespace.ok_or_else(|| {
        syn::Error::new_spanned(&input.ident, "missing #[sigmf(namespace = \"...\")]")
    })?;
    let version = version.ok_or_else(|| {
        syn::Error::new_spanned(&input.ident, "missing #[sigmf(version = \"...\")]")
    })?;
    let scope = scope.ok_or_else(|| {
        syn::Error::new_spanned(&input.ident, "missing #[sigmf(scope = \"...\")]")
    })?;

    let namespace_value = namespace.value();
    if namespace_value == "core"
        || namespace_value.is_empty()
        || !namespace_value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
    {
        return Err(syn::Error::new_spanned(
            namespace,
            "namespace must be a non-empty ASCII identifier and cannot be \"core\"",
        ));
    }
    if version.value().is_empty() {
        return Err(syn::Error::new_spanned(
            version,
            "version must not be empty",
        ));
    }

    let scope_variant = match scope.value().as_str() {
        "global" => quote!(Global),
        "capture" => quote!(Capture),
        "annotation" => quote!(Annotation),
        "collection" => quote!(Collection),
        _ => {
            return Err(syn::Error::new_spanned(
                scope,
                "scope must be one of global, capture, annotation, or collection",
            ));
        }
    };

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &input.ident,
                    "SigmfExtension requires a struct with named fields",
                ));
            }
        },
        _ => {
            return Err(syn::Error::new_spanned(
                &input.ident,
                "SigmfExtension can only be derived for structs",
            ));
        }
    };

    reject_conflicting_container_attributes(&input.attrs)?;
    let mut names = Vec::<LitStr>::new();
    let mut seen = std::collections::HashSet::new();
    for field in fields {
        let wire_name = serde_wire_name(&field.attrs)?;
        let wire_value = wire_name.value();
        let expected_prefix = format!("{namespace_value}:");
        if !wire_value.starts_with(&expected_prefix) || wire_value.len() == expected_prefix.len() {
            return Err(syn::Error::new_spanned(
                wire_name,
                format!("extension fields must use the {expected_prefix:?} namespace"),
            ));
        }
        if !seen.insert(wire_value.clone()) {
            return Err(syn::Error::new_spanned(
                wire_name,
                "duplicate extension wire field name",
            ));
        }
        names.push(wire_name);
    }

    let ident = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    Ok(quote! {
        impl #impl_generics #crate_path::extensions::SigmfExtension
            for #ident #ty_generics #where_clause
        {
            const NAMESPACE: &'static str = #namespace_value;
            const VERSION: &'static str = #version;
            const SCOPE: #crate_path::extensions::ExtensionScope =
                #crate_path::extensions::ExtensionScope::#scope_variant;
            const FIELDS: &'static [&'static str] = &[#(#names),*];
        }
    })
}

fn reject_conflicting_container_attributes(attributes: &[Attribute]) -> Result<()> {
    for attribute in attributes
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
    {
        attribute.parse_nested_meta(|meta| {
            if [
                "rename_all",
                "rename_all_fields",
                "transparent",
                "tag",
                "content",
                "untagged",
                "from",
                "try_from",
            ]
            .iter()
            .any(|unsupported| meta.path.is_ident(unsupported))
            {
                return Err(
                    meta.error("this Serde container attribute is not supported by SigmfExtension")
                );
            }
            if meta.input.peek(syn::Token![=]) {
                let _: syn::Expr = meta.value()?.parse()?;
            }
            Ok(())
        })?;
    }
    Ok(())
}

fn serde_wire_name(attributes: &[Attribute]) -> Result<LitStr> {
    let mut wire_name = None;
    for attribute in attributes
        .iter()
        .filter(|attr| attr.path().is_ident("serde"))
    {
        attribute.parse_nested_meta(|meta| {
            if meta.path.is_ident("rename") {
                if wire_name.is_some() {
                    return Err(meta.error("multiple Serde rename attributes are unsupported"));
                }
                wire_name = Some(meta.value()?.parse::<LitStr>()?);
            } else if [
                "flatten",
                "with",
                "serialize_with",
                "deserialize_with",
                "skip",
                "skip_serializing",
                "skip_deserializing",
            ]
            .iter()
            .any(|unsupported| meta.path.is_ident(unsupported))
            {
                return Err(
                    meta.error("this Serde field attribute is not supported by SigmfExtension")
                );
            }
            if meta.input.peek(syn::Token![=]) {
                let _: syn::Expr = meta.value()?.parse()?;
            }
            Ok(())
        })?;
    }
    wire_name.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "each extension field requires an explicit #[serde(rename = \"namespace:field\")]",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic(source: &str) -> String {
        let input = syn::parse_str::<DeriveInput>(source).unwrap();
        expand(input).unwrap_err().to_string()
    }

    #[test]
    fn reports_missing_and_reserved_extension_attributes() {
        assert!(diagnostic("struct Missing;").contains("missing #[sigmf(namespace"));
        let error = diagnostic(
            r#"
                #[sigmf(namespace = "core", version = "1.0.0", scope = "global")]
                struct Reserved {
                    #[serde(rename = "core:value")]
                    value: bool,
                }
            "#,
        );
        assert!(error.contains("cannot be \"core\""));
    }

    #[test]
    fn rejects_invalid_wire_shapes_with_field_diagnostics() {
        let error = diagnostic(
            r#"
                #[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
                struct WrongNamespace {
                    #[serde(rename = "other:value")]
                    value: bool,
                }
            "#,
        );
        assert!(error.contains("must use the \"lab:\" namespace"));

        let error = diagnostic(
            r#"
                #[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
                struct Flattened {
                    #[serde(rename = "lab:value", flatten)]
                    value: bool,
                }
            "#,
        );
        assert!(error.contains("not supported by SigmfExtension"));

        let error = diagnostic(
            r#"
                #[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
                struct Skipped {
                    #[serde(rename = "lab:value", skip)]
                    value: bool,
                }
            "#,
        );
        assert!(error.contains("not supported by SigmfExtension"));
    }

    #[test]
    fn rejects_unsupported_derive_shapes() {
        let error = diagnostic(
            r#"
                #[sigmf(namespace = "lab", version = "1.0.0", scope = "capture")]
                enum Unsupported {
                    Value,
                }
            "#,
        );
        assert!(error.contains("can only be derived for structs"));

        let error = diagnostic(
            r#"
                #[sigmf(namespace = "lab", namespace = "vendor", version = "1.0.0", scope = "global")]
                struct DuplicateNamespace {
                    #[serde(rename = "lab:value")]
                    value: bool,
                }
            "#,
        );
        assert!(error.contains("duplicate sigmf namespace attribute"));
    }
}
