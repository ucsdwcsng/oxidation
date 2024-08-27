use crate::global;
use bytemuck::{self};
use num_complex::Complex;

#[derive(Debug)]
pub enum SampleData {
    ComplexUnsignedBytes(Vec<Complex<u8>>),
    ComplexSignedBytes(Vec<Complex<i8>>),
    ComplexF32(Vec<Complex<f32>>),
    ComplexF64(Vec<Complex<f64>>),
    ComplexI16(Vec<Complex<i16>>),
    ComplexI32(Vec<Complex<i32>>),
    ComplexU16(Vec<Complex<u16>>),
    ComplexU32(Vec<Complex<u32>>),
    RealUnsignedBytes(Vec<u8>),
    RealSignedBytes(Vec<i8>),
    RealF32(Vec<f32>),
    RealF64(Vec<f64>),
    RealI16(Vec<i16>),
    RealI32(Vec<i32>),
    RealU16(Vec<u16>),
    RealU32(Vec<u32>),
}

pub fn handle_complex_bytes(data: &[u8], byte_type: global::SmfDataTypeByte) -> SampleData {
    match byte_type {
        global::SmfDataTypeByte::U8 => {
            let data_complex: Vec<Complex<u8>> = unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const Complex<u8>, data.len() / 2)
            }
            .to_vec();
            SampleData::ComplexUnsignedBytes(data_complex)
        }
        global::SmfDataTypeByte::I8 => {
            let data_i8: Vec<i8> = data.iter().map(|&x| x as i8).collect();
            let data_complex: Vec<Complex<i8>> = unsafe {
                std::slice::from_raw_parts(
                    data_i8.as_ptr() as *const Complex<i8>,
                    data_i8.len() / 2,
                )
            }
            .to_vec();
            SampleData::ComplexSignedBytes(data_complex)
        }
    }
}
pub fn handle_complex_data(
    data: &[u8],
    dtype: global::SmfDataTypeDtype,
    endian: global::SmfDataTypeEndian,
) -> SampleData {
    match (dtype, endian) {
        (global::SmfDataTypeDtype::F32, global::SmfDataTypeEndian::LE) => {
            let data_f32: &[f32] = bytemuck::cast_slice(data);
            let len: usize = data_f32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<f32>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_f32[re_idx], data_f32[im_idx]));
                }

                SampleData::ComplexF32(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::F64, global::SmfDataTypeEndian::LE) => {
            let data_f64: &[f64] = bytemuck::cast_slice(data);
            let len: usize = data_f64.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<f64>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_f64[re_idx], data_f64[im_idx]));
                }

                SampleData::ComplexF64(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::I16, global::SmfDataTypeEndian::LE) => {
            let data_i16: &[i16] = bytemuck::cast_slice(data);
            let len: usize = data_i16.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<i16>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_i16[re_idx], data_i16[im_idx]));
                }

                SampleData::ComplexI16(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::I32, global::SmfDataTypeEndian::LE) => {
            let data_i32: &[i32] = bytemuck::cast_slice(data);
            let len: usize = data_i32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<i32>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_i32[re_idx], data_i32[im_idx]));
                }

                SampleData::ComplexI32(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::U16, global::SmfDataTypeEndian::LE) => {
            let data_u16: &[u16] = bytemuck::cast_slice(data);
            let len: usize = data_u16.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<u16>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_u16[re_idx], data_u16[im_idx]));
                }

                SampleData::ComplexU16(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::U32, global::SmfDataTypeEndian::LE) => {
            let data_u32: &[u32] = bytemuck::cast_slice(data);
            let len: usize = data_u32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<u32>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_u32[re_idx], data_u32[im_idx]));
                }

                SampleData::ComplexU32(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::F32, global::SmfDataTypeEndian::BE) => {
            let data_f32: &[f32] = bytemuck::cast_slice(data);
            let len: usize = data_f32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<f32>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_f32[re_idx], data_f32[im_idx]));
                }

                SampleData::ComplexF32(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::F64, global::SmfDataTypeEndian::BE) => {
            let data_f64: &[f64] = bytemuck::cast_slice(data);
            let len: usize = data_f64.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<f64>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_f64[re_idx], data_f64[im_idx]));
                }

                SampleData::ComplexF64(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::I16, global::SmfDataTypeEndian::BE) => {
            let data_i16: &[i16] = bytemuck::cast_slice(data);
            let len: usize = data_i16.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<i16>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_i16[re_idx], data_i16[im_idx]));
                }

                SampleData::ComplexI16(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::I32, global::SmfDataTypeEndian::BE) => {
            let data_i32: &[i32] = bytemuck::cast_slice(data);
            let len: usize = data_i32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<i32>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_i32[re_idx], data_i32[im_idx]));
                }

                SampleData::ComplexI32(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::U16, global::SmfDataTypeEndian::BE) => {
            let data_u16: &[u16] = bytemuck::cast_slice(data);
            let len: usize = data_u16.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<u16>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_u16[re_idx], data_u16[im_idx]));
                }

                SampleData::ComplexU16(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::U32, global::SmfDataTypeEndian::BE) => {
            let data_u32: &[u32] = bytemuck::cast_slice(data);
            let len: usize = data_u32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<Complex<u32>> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(Complex::new(data_u32[re_idx], data_u32[im_idx]));
                }

                SampleData::ComplexU32(data_complex)
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
    }
}
pub fn handle_real_bytes(data: &[u8], byte_type: global::SmfDataTypeByte) -> SampleData {
    match byte_type {
        global::SmfDataTypeByte::U8 => {
            let data_real: Vec<u8> =
                unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() / 2) }
                    .to_vec();
            SampleData::RealUnsignedBytes(data_real)
        }
        global::SmfDataTypeByte::I8 => {
            let data_i8: Vec<i8> = data.iter().map(|&x| x as i8).collect();
            let data_real: Vec<i8> = unsafe {
                std::slice::from_raw_parts(data_i8.as_ptr() as *const i8, data_i8.len() / 2)
            }
            .to_vec();
            SampleData::RealSignedBytes(data_real)
        }
    }
}
pub fn handle_real_data(
    data: &[u8],
    dtype: global::SmfDataTypeDtype,
    endian: global::SmfDataTypeEndian,
) -> SampleData {
    match (dtype, endian) {
        (global::SmfDataTypeDtype::F32, global::SmfDataTypeEndian::LE) => {
            let data_f32: &[f32] = bytemuck::cast_slice(data);
            SampleData::RealF32(data_f32.to_vec())
        }
        (global::SmfDataTypeDtype::F64, global::SmfDataTypeEndian::LE) => {
            let data_f64: &[f64] = bytemuck::cast_slice(data);
            SampleData::RealF64(data_f64.to_vec())
        }
        (global::SmfDataTypeDtype::I16, global::SmfDataTypeEndian::LE) => {
            let data_i16: &[i16] = bytemuck::cast_slice(data);
            SampleData::RealI16(data_i16.to_vec())
        }
        (global::SmfDataTypeDtype::I32, global::SmfDataTypeEndian::LE) => {
            let data_i32: &[i32] = bytemuck::cast_slice(data);
            SampleData::RealI32(data_i32.to_vec())
        }
        (global::SmfDataTypeDtype::U16, global::SmfDataTypeEndian::LE) => {
            let data_u16: &[u16] = bytemuck::cast_slice(data);
            SampleData::RealU16(data_u16.to_vec())
        }
        (global::SmfDataTypeDtype::U32, global::SmfDataTypeEndian::LE) => {
            let data_u32: &[u32] = bytemuck::cast_slice(data);
            SampleData::RealU32(data_u32.to_vec())
        }
        (global::SmfDataTypeDtype::F32, global::SmfDataTypeEndian::BE) => {
            let data_f32: &[f32] = bytemuck::cast_slice(data);
            SampleData::RealF32(data_f32.to_vec())
        }
        (global::SmfDataTypeDtype::F64, global::SmfDataTypeEndian::BE) => {
            let data_f64: &[f64] = bytemuck::cast_slice(data);
            SampleData::RealF64(data_f64.to_vec())
        }
        (global::SmfDataTypeDtype::I16, global::SmfDataTypeEndian::BE) => {
            let data_i16: &[i16] = bytemuck::cast_slice(data);
            SampleData::RealI16(data_i16.to_vec())
        }
        (global::SmfDataTypeDtype::I32, global::SmfDataTypeEndian::BE) => {
            let data_i32: &[i32] = bytemuck::cast_slice(data);
            SampleData::RealI32(data_i32.to_vec())
        }
        (global::SmfDataTypeDtype::U16, global::SmfDataTypeEndian::BE) => {
            let data_u16: &[u16] = bytemuck::cast_slice(data);
            SampleData::RealU16(data_u16.to_vec())
        }
        (global::SmfDataTypeDtype::U32, global::SmfDataTypeEndian::BE) => {
            let data_u32: &[u32] = bytemuck::cast_slice(data);
            SampleData::RealU32(data_u32.to_vec())
        }
    }
}
