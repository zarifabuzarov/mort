use std::io::Cursor;
// use image::{ImageFormat};.
use wasmtime::{Caller, Linker};

use crate::state::HostState;
use crate::utils::{with_mem_slice, write_bytes_to_mem};
use image::{ImageReader, ImageBuffer, Rgba, ImageFormat};

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ImageTransform {
    pub resize_width: u32,
    pub resize_height: u32,
    pub rotate_degrees: u16,
    pub crop_x: u32,
    pub crop_y: u32,
    pub crop_w: u32,
    pub crop_h: u32,
}

pub fn register_media_api(linker: &mut Linker<HostState>) -> anyhow::Result<()> {
    linker.func_wrap(
        "env",
        "host_image_info",
        |mut caller: Caller<'_, HostState>,
         img_ptr: i32,
         img_len: i32,
         out_width_ptr: i32,
         out_height_ptr: i32| -> i32 {
            let res = with_mem_slice(&mut caller, img_ptr, img_len, |img_bytes| {
                let Ok(reader) = ImageReader::new(Cursor::new(img_bytes)).with_guessed_format() else {
                    return Err(-2);
                };

                let Ok((width, height)) = reader.into_dimensions() else {
                    return Err(-3);
                };

                Ok((width, height))
            });

            let (width, height) = match res {
                Ok(Ok((w, h))) => (w, h),
                Ok(Err(code)) => return code,
                Err(_) => return -1,
            };

            if write_bytes_to_mem(&mut caller, out_width_ptr, &width.to_le_bytes()).is_err() {
                return -4;
            }
            if write_bytes_to_mem(&mut caller, out_height_ptr, &height.to_le_bytes()).is_err() {
                return -4;
            }

            0
        },
    )?;

    linker.func_wrap(
        "env",
        "host_decode_image",
        |mut caller: Caller<'_, HostState>,
         img_ptr: i32,
         img_len: i32,
         out_rgba_ptr: i32,
         out_max_len: i32| -> i32 {
            let raw_pixels = match with_mem_slice(&mut caller, img_ptr, img_len, |img_bytes| {
                let Ok(img) = image::load_from_memory(img_bytes) else {
                    return Err(-2);
                };
                Ok(img.to_rgba8().into_raw())
            }) {
                Ok(Ok(pixels)) => pixels,
                Ok(Err(code)) => return code,
                Err(_) => return -1,
            };

            if raw_pixels.len() > out_max_len as usize {
                return -3;
            }

            if write_bytes_to_mem(&mut caller, out_rgba_ptr, &raw_pixels).is_err() {
                return -4;
            }

            raw_pixels.len() as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "host_encode_image",
        |mut caller: Caller<'_, HostState>,
         rgba_ptr: i32,
         rgba_len: i32,
         width: u32,
         height: u32,
         format_code: u32,
         out_ptr: i32,
         out_max_len: i32| -> i32 {

            // Проверяем, что размер массива пикселей совпадает с (w * h * 4)
            if rgba_len as usize != (width * height * 4) as usize {
                return -1; // Неверный размер RGBA буфера
            }

            let encoded_result = match with_mem_slice(&mut caller, rgba_ptr, rgba_len, |pixels| {
                // Создаем картинку из сырых RGBA байт
                let Some(img_buf) = ImageBuffer::<Rgba<u8>, _>::from_raw(width, height, pixels) else {
                    return Err(-2);
                };

                let target_format = match format_code {
                    0 => ImageFormat::Jpeg,
                    1 => ImageFormat::Png,
                    2 => ImageFormat::WebP,
                    _ => ImageFormat::Jpeg,
                };

                let mut out = Vec::new();
                if img_buf.write_to(&mut Cursor::new(&mut out), target_format).is_err() {
                    return Err(-3);
                }

                Ok(out)
            }) {
                Ok(Ok(bytes)) => bytes,
                Ok(Err(code)) => return code,
                Err(_) => return -4,
            };

            if encoded_result.len() > out_max_len as usize {
                return -5; // Выходной буфер слишком мал
            }

            if write_bytes_to_mem(&mut caller, out_ptr, &encoded_result).is_err() {
                return -6;
            }

            encoded_result.len() as i32
        },
    )?;

    linker.func_wrap(
        "env",
        "host_process_image",
        |mut caller: Caller<'_, HostState>,
         src_ptr: i32,
         src_len: i32,
         transform_ptr: i32,
         out_ptr: i32,
         out_max_len: i32| -> i32 {
            let transform = match with_mem_slice(
                &mut caller,
                transform_ptr,
                std::mem::size_of::<ImageTransform>() as i32,
                |slice| unsafe {
                    std::ptr::read_unaligned(slice.as_ptr() as *const ImageTransform)
                },
            ) {
                Ok(t) => t,
                Err(_) => return -1,
            };

            let encoded_result = match with_mem_slice(&mut caller, src_ptr, src_len, |src_bytes| {
                let Ok(mut img) = image::load_from_memory(src_bytes) else {
                    return Err(-3);
                };

                if transform.crop_w > 0 && transform.crop_h > 0 {
                    img = img.crop_imm(
                        transform.crop_x,
                        transform.crop_y,
                        transform.crop_w,
                        transform.crop_h,
                    );
                }

                match transform.rotate_degrees {
                    90 => img = img.rotate90(),
                    180 => img = img.rotate180(),
                    270 => img = img.rotate270(),
                    _ => {}
                }

                if transform.resize_width > 0 && transform.resize_height > 0 {
                    img = img.resize_exact(
                        transform.resize_width,
                        transform.resize_height,
                        image::imageops::FilterType::Triangle,
                    );
                }

                let mut out = Vec::new();
                if img.write_to(&mut Cursor::new(&mut out), ImageFormat::Jpeg).is_err() {
                    return Err(-4);
                }

                Ok(out)
            }) {
                Ok(Ok(bytes)) => bytes,
                Ok(Err(code)) => return code,
                Err(_) => return -2,
            };

            if encoded_result.len() > out_max_len as usize {
                return -5;
            }

            if write_bytes_to_mem(&mut caller, out_ptr, &encoded_result).is_err() {
                return -6;
            }

            encoded_result.len() as i32
        },
    )?;

    Ok(())
}