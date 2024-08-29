use crate::global;
use bytemuck::{self};
use num_complex::Complex;

#[derive(Debug)]
pub enum DataSample {
    ComplexUnsignedBytes(Complex<u8>),
    ComplexSignedBytes(Complex<i8>),
    ComplexF32(Complex<f32>),
    ComplexF64(Complex<f64>),
    ComplexI16(Complex<i16>),
    ComplexI32(Complex<i32>),
    ComplexU16(Complex<u16>),
    ComplexU32(Complex<u32>),
    RealUnsignedBytes(u8),
    RealSignedBytes(i8),
    RealF32(f32),
    RealF64(f64),
    RealI16(i16),
    RealI32(i32),
    RealU16(u16),
    RealU32(u32),
}

pub fn handle_complex_bytes(data: &[u8], byte_type: global::SmfDataTypeByte) -> Vec<DataSample> {
    match byte_type {
        global::SmfDataTypeByte::U8 => {
            let data_complex: Vec<Complex<u8>> = unsafe {
                std::slice::from_raw_parts(data.as_ptr() as *const Complex<u8>, data.len() / 2)
            }
            .to_vec();
            data_complex
                .iter()
                .map(|&x| DataSample::ComplexUnsignedBytes(x))
                .collect()
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
            data_complex
                .iter()
                .map(|&x| DataSample::ComplexSignedBytes(x))
                .collect()
        }
    }
}

pub fn handle_complex_data(
    data: &[u8],
    dtype: global::SmfDataTypeDtype,
    endian: global::SmfDataTypeEndian,
) -> Vec<DataSample> {
    match (dtype, endian) {
        (global::SmfDataTypeDtype::F32, global::SmfDataTypeEndian::LE) => {
            let data_f32: &[f32] = bytemuck::cast_slice(data);
            let len: usize = data_f32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexF32(Complex::new(
                        data_f32[re_idx],
                        data_f32[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::F64, global::SmfDataTypeEndian::LE) => {
            let data_f64: &[f64] = bytemuck::cast_slice(data);
            let len: usize = data_f64.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexF64(Complex::new(
                        data_f64[re_idx],
                        data_f64[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::I16, global::SmfDataTypeEndian::LE) => {
            let data_i16: &[i16] = bytemuck::cast_slice(data);
            let len: usize = data_i16.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexI16(Complex::new(
                        data_i16[re_idx],
                        data_i16[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::I32, global::SmfDataTypeEndian::LE) => {
            let data_i32: &[i32] = bytemuck::cast_slice(data);
            let len: usize = data_i32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexI32(Complex::new(
                        data_i32[re_idx],
                        data_i32[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::U16, global::SmfDataTypeEndian::LE) => {
            let data_u16: &[u16] = bytemuck::cast_slice(data);
            let len: usize = data_u16.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexU16(Complex::new(
                        data_u16[re_idx],
                        data_u16[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::U32, global::SmfDataTypeEndian::LE) => {
            let data_u32: &[u32] = bytemuck::cast_slice(data);
            let len: usize = data_u32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexU32(Complex::new(
                        data_u32[re_idx],
                        data_u32[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::F32, global::SmfDataTypeEndian::BE) => {
            let data_f32: &[f32] = bytemuck::cast_slice(data);
            let len: usize = data_f32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexF32(Complex::new(
                        data_f32[re_idx],
                        data_f32[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::F64, global::SmfDataTypeEndian::BE) => {
            let data_f64: &[f64] = bytemuck::cast_slice(data);
            let len: usize = data_f64.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexF64(Complex::new(
                        data_f64[re_idx],
                        data_f64[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::I16, global::SmfDataTypeEndian::BE) => {
            let data_i16: &[i16] = bytemuck::cast_slice(data);
            let len: usize = data_i16.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexI16(Complex::new(
                        data_i16[re_idx],
                        data_i16[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::I32, global::SmfDataTypeEndian::BE) => {
            let data_i32: &[i32] = bytemuck::cast_slice(data);
            let len: usize = data_i32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexI32(Complex::new(
                        data_i32[re_idx],
                        data_i32[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::U16, global::SmfDataTypeEndian::BE) => {
            let data_u16: &[u16] = bytemuck::cast_slice(data);
            let len: usize = data_u16.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexU16(Complex::new(
                        data_u16[re_idx],
                        data_u16[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
        (global::SmfDataTypeDtype::U32, global::SmfDataTypeEndian::BE) => {
            let data_u32: &[u32] = bytemuck::cast_slice(data);
            let len: usize = data_u32.len();

            if len % 2 == 0 {
                let mut data_complex: Vec<DataSample> = Vec::with_capacity(len / 2);
                for i in 0..len / 2 {
                    let re_idx = i * 2;
                    let im_idx = i * 2 + 1;
                    data_complex.push(DataSample::ComplexU32(Complex::new(
                        data_u32[re_idx],
                        data_u32[im_idx],
                    )));
                }

                data_complex
            } else {
                panic!("Data length is not an even number, cannot be grouped into complex numbers");
            }
        }
    }
}

pub fn handle_real_bytes(data: &[u8], byte_type: global::SmfDataTypeByte) -> Vec<DataSample> {
    match byte_type {
        global::SmfDataTypeByte::U8 => {
            let data_real: Vec<u8> =
                unsafe { std::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() / 2) }
                    .to_vec();
            data_real
                .iter()
                .map(|&x| DataSample::RealUnsignedBytes(x))
                .collect()
        }
        global::SmfDataTypeByte::I8 => {
            let data_i8: Vec<i8> = data.iter().map(|&x| x as i8).collect();
            let data_real: Vec<i8> = unsafe {
                std::slice::from_raw_parts(data_i8.as_ptr() as *const i8, data_i8.len() / 2)
            }
            .to_vec();
            data_real
                .iter()
                .map(|&x| DataSample::RealSignedBytes(x))
                .collect()
        }
    }
}

pub fn handle_real_data(
    data: &[u8],
    dtype: global::SmfDataTypeDtype,
    endian: global::SmfDataTypeEndian,
) -> Vec<DataSample> {
    match (dtype, endian) {
        (global::SmfDataTypeDtype::F32, global::SmfDataTypeEndian::LE) => {
            let data_f32: &[f32] = bytemuck::cast_slice(data);
            data_f32.iter().map(|&x| DataSample::RealF32(x)).collect()
        }
        (global::SmfDataTypeDtype::F64, global::SmfDataTypeEndian::LE) => {
            let data_f64: &[f64] = bytemuck::cast_slice(data);
            data_f64.iter().map(|&x| DataSample::RealF64(x)).collect()
        }
        (global::SmfDataTypeDtype::I16, global::SmfDataTypeEndian::LE) => {
            let data_i16: &[i16] = bytemuck::cast_slice(data);
            data_i16.iter().map(|&x| DataSample::RealI16(x)).collect()
        }
        (global::SmfDataTypeDtype::I32, global::SmfDataTypeEndian::LE) => {
            let data_i32: &[i32] = bytemuck::cast_slice(data);
            data_i32.iter().map(|&x| DataSample::RealI32(x)).collect()
        }
        (global::SmfDataTypeDtype::U16, global::SmfDataTypeEndian::LE) => {
            let data_u16: &[u16] = bytemuck::cast_slice(data);
            data_u16.iter().map(|&x| DataSample::RealU16(x)).collect()
        }
        (global::SmfDataTypeDtype::U32, global::SmfDataTypeEndian::LE) => {
            let data_u32: &[u32] = bytemuck::cast_slice(data);
            data_u32.iter().map(|&x| DataSample::RealU32(x)).collect()
        }
        (global::SmfDataTypeDtype::F32, global::SmfDataTypeEndian::BE) => {
            let data_f32: &[f32] = bytemuck::cast_slice(data);
            data_f32.iter().map(|&x| DataSample::RealF32(x)).collect()
        }
        (global::SmfDataTypeDtype::F64, global::SmfDataTypeEndian::BE) => {
            let data_f64: &[f64] = bytemuck::cast_slice(data);
            data_f64.iter().map(|&x| DataSample::RealF64(x)).collect()
        }
        (global::SmfDataTypeDtype::I16, global::SmfDataTypeEndian::BE) => {
            let data_i16: &[i16] = bytemuck::cast_slice(data);
            data_i16.iter().map(|&x| DataSample::RealI16(x)).collect()
        }
        (global::SmfDataTypeDtype::I32, global::SmfDataTypeEndian::BE) => {
            let data_i32: &[i32] = bytemuck::cast_slice(data);
            data_i32.iter().map(|&x| DataSample::RealI32(x)).collect()
        }
        (global::SmfDataTypeDtype::U16, global::SmfDataTypeEndian::BE) => {
            let data_u16: &[u16] = bytemuck::cast_slice(data);
            data_u16.iter().map(|&x| DataSample::RealU16(x)).collect()
        }
        (global::SmfDataTypeDtype::U32, global::SmfDataTypeEndian::BE) => {
            let data_u32: &[u32] = bytemuck::cast_slice(data);
            data_u32.iter().map(|&x| DataSample::RealU32(x)).collect()
        }
    }
}

