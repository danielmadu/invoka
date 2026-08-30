//! Icon resolution for application entries.
//!
//! Windows: extract the icon of a shortcut target via the shell (`SHGFI_ICON`),
//! convert the `HICON` to 32-bit BGRA with GDI and encode it as PNG in a
//! per-process cache (`%APPDATA%\invoka\icons`). Linux: the `.desktop` scan
//! resolves icons through the freedesktop icon theme instead.

#[cfg(windows)]
use std::path::PathBuf;

/// Resolve a cached PNG icon for `target`, extracting it on first use.
#[cfg(windows)]
pub fn icon_for(target: &std::path::Path) -> Option<PathBuf> {
    cache_path(target).or_else(|| extract_and_cache(target))
}

/// Cache location for an icon (`<cache_dir>\<hash of target>.png`).
#[cfg(windows)]
fn cache_path(target: &std::path::Path) -> Option<PathBuf> {
    let dir = cache_dir()?;
    let path = dir.join(format!("{}-{}.png", hash_target(target), size_hint(target)));
    path.is_file().then_some(path)
}

/// Cache directory (`%APPDATA%\invoka\icons`).
#[cfg(windows)]
pub fn cache_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(
        PathBuf::from(appdata)
            .join("invoka")
            .join("icons"),
    )
}

/// Stable cache key: FNV-1a over the lowercase target path.
#[cfg_attr(not(windows), allow(dead_code))]
fn hash_target(target: &std::path::Path) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in target.to_string_lossy().to_lowercase().as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// Target size in the cache key so different icon sizes don't collide.
#[cfg(windows)]
fn size_hint(target: &std::path::Path) -> u64 {
    std::fs::metadata(target).map(|m| m.len()).unwrap_or(0)
}

#[cfg(windows)]
mod extract {
    use std::path::{Path, PathBuf};

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::Graphics::Gdi::{
        DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO,
        BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    };
    use windows_sys::Win32::Storage::FileSystem::FILE_ATTRIBUTE_NORMAL;
    use windows_sys::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON};
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, ICONINFO};

    use super::{cache_dir, hash_target, size_hint};

    /// Extract the shell icon of `target` and write a PNG to the cache.
    pub fn extract(target: &Path) -> Option<PathBuf> {
        let out_dir = cache_dir()?;
        let out_path = out_dir.join(format!(
            "{}-{}.png",
            hash_target(target),
            size_hint(target)
        ));

        let hicon = shell_icon(target)?;
        let result = hicon_to_png(hicon, &out_path);
        unsafe { DestroyIcon(hicon) };
        result.then_some(out_path)
    }

    fn to_wide(value: &str) -> Vec<u16> {
        use std::os::windows::ffi::OsStrExt;
        std::ffi::OsStr::new(value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    fn shell_icon(target: &Path) -> Option<HANDLE> {
        let wide = to_wide(&target.to_string_lossy());
        let mut info: SHFILEINFOW = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            SHGetFileInfoW(
                wide.as_ptr(),
                FILE_ATTRIBUTE_NORMAL,
                &mut info,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON,
            )
        };
        (ok != 0 && !info.hIcon.is_null()).then_some(info.hIcon)
    }

    /// Convert an `HICON` to a top-down RGBA buffer and encode it as PNG.
    fn hicon_to_png(hicon: HANDLE, out_path: &Path) -> bool {
        let mut icon_info: ICONINFO = unsafe { std::mem::zeroed() };
        if unsafe { GetIconInfo(hicon, &mut icon_info) } == 0 {
            return false;
        }
        let hbmp_color = icon_info.hbmColor;
        let hbmp_mask = icon_info.hbmMask;

        let ok = with_bitmap_size(hbmp_color, |width, height| {
            if width == 0 || height == 0 {
                return false;
            }

            // Negative height = top-down rows.
            let mut bmi: BITMAPINFO = unsafe { std::mem::zeroed() };
            bmi.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = width;
            bmi.bmiHeader.biHeight = -(height as i32);
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB;

            let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
            let dc = unsafe { GetDC(std::ptr::null_mut()) };
            let lines = unsafe {
                GetDIBits(
                    dc,
                    hbmp_color,
                    0,
                    height as u32,
                    pixels.as_mut_ptr().cast(),
                    &mut bmi,
                    DIB_RGB_COLORS,
                )
            };
            unsafe { ReleaseDC(std::ptr::null_mut(), dc) };
            if lines != height as i32 {
                return false;
            }

            // GDI gives BGRA; flip to RGBA. Icons rendered without an alpha
            // channel report fully opaque backgrounds only after the mask is
            // applied — treat all-zero alpha as opaque.
            let mut has_alpha = false;
            for px in pixels.chunks_exact_mut(4) {
                px.swap(0, 2);
                has_alpha |= px[3] != 0;
            }
            if !has_alpha {
                for px in pixels.chunks_exact_mut(4) {
                    px[3] = 255;
                }
            }

            let Some(image) = image::RgbaImage::from_raw(width as u32, height as u32, pixels)
            else {
                return false;
            };
            save_png(&image, out_path)
        });

        unsafe { DeleteObject(hbmp_color) };
        unsafe { DeleteObject(hbmp_mask) };
        ok
    }

    fn with_bitmap_size(bitmap: windows_sys::Win32::Graphics::Gdi::HBITMAP, f: impl FnOnce(i32, i32) -> bool) -> bool {
        let mut bitmap_info: BITMAP = unsafe { std::mem::zeroed() };
        let read = std::mem::size_of::<BITMAP>() as i32;
        if unsafe { GetObjectW(bitmap, read, (&mut bitmap_info as *mut BITMAP).cast()) } == 0 {
            return false;
        }
        f(bitmap_info.bmWidth, bitmap_info.bmHeight)
    }

    fn save_png(image: &image::RgbaImage, out_path: &Path) -> bool {
        if let Some(parent) = out_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let Ok(file) = std::fs::File::create(out_path) else {
            return false;
        };
        let mut writer = std::io::BufWriter::new(file);
        image.write_to(&mut writer, image::ImageFormat::Png).is_ok()
    }
}

#[cfg(windows)]
use extract::extract;

/// Extract-and-cache fallback when the icon isn't cached yet.
#[cfg(windows)]
fn extract_and_cache(target: &std::path::Path) -> Option<PathBuf> {
    extract(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable_and_case_insensitive() {
        use std::path::Path;
        let a = Path::new(r"C:\Program Files\App\app.exe");
        let b = Path::new(r"c:\PROGRAM FILES\APP\APP.EXE");
        assert_eq!(hash_target(a), hash_target(b));
        assert_ne!(hash_target(a), hash_target(Path::new(r"C:\other\app.exe")));
    }
}
