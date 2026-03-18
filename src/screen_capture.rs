use std::ptr;

const SRCCOPY: u32 = 0x00CC0020;
const BI_RGB: u32 = 0;
const DIB_RGB_COLORS: u32 = 0;
const SM_CXSCREEN: i32 = 0;
const SM_CYSCREEN: i32 = 1;

#[repr(C)]
pub struct BitmapInfoHeader {
    pub bi_size: u32,
    pub bi_width: i32,
    pub bi_height: i32,
    pub bi_planes: u16,
    pub bi_bit_count: u16,
    pub bi_compression: u32,
    pub bi_size_image: u32,
    pub bi_x_pels_per_meter: i32,
    pub bi_y_pels_per_meter: i32,
    pub bi_clr_used: u32,
    pub bi_clr_important: u32,
}

#[repr(C)]
pub struct BitmapInfo {
    pub bmi_header: BitmapInfoHeader,
    pub bmi_colors: [u32; 1],
}

type CreateCompatibleDCFn = unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void;
type CreateCompatibleBitmapFn = unsafe extern "system" fn(*mut std::ffi::c_void, i32, i32) -> *mut std::ffi::c_void;
type SelectObjectFn = unsafe extern "system" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> *mut std::ffi::c_void;
type BitBltFn = unsafe extern "system" fn(*mut std::ffi::c_void, i32, i32, i32, i32, *mut std::ffi::c_void, i32, i32, u32) -> i32;
type GetDIBitsFn = unsafe extern "system" fn(*mut std::ffi::c_void, *mut std::ffi::c_void, u32, u32, *mut u8, *mut BitmapInfo, u32) -> i32;
type DeleteObjectFn = unsafe extern "system" fn(*mut std::ffi::c_void) -> i32;
type DeleteDCFn = unsafe extern "system" fn(*mut std::ffi::c_void) -> i32;
type GetDCFn = unsafe extern "system" fn(*mut std::ffi::c_void) -> *mut std::ffi::c_void;
type ReleaseDCFn = unsafe extern "system" fn(*mut std::ffi::c_void, *mut std::ffi::c_void) -> i32;
type GetSystemMetricsFn = unsafe extern "system" fn(i32) -> i32;

// You can use this without lazy static for more readable code but i dont think its needed.

lazy_static::lazy_static! {
    static ref DYN_GDI32: libloading::Library = unsafe { libloading::Library::new(crate::obf_string!("gdi32.dll")).expect("failed to load gdi32.dll") };
    static ref DYN_USER32_SC: libloading::Library = unsafe { libloading::Library::new(crate::obf_string!("user32.dll")).expect("failed to load user32.dll") };

    pub static ref DYN_CREATE_COMPATIBLE_DC: libloading::Symbol<'static, CreateCompatibleDCFn> = unsafe { DYN_GDI32.get(b"CreateCompatibleDC\0").unwrap() };
    pub static ref DYN_CREATE_COMPATIBLE_BITMAP: libloading::Symbol<'static, CreateCompatibleBitmapFn> = unsafe { DYN_GDI32.get(b"CreateCompatibleBitmap\0").unwrap() };
    pub static ref DYN_SELECT_OBJECT: libloading::Symbol<'static, SelectObjectFn> = unsafe { DYN_GDI32.get(b"SelectObject\0").unwrap() };
    pub static ref DYN_BIT_BLT: libloading::Symbol<'static, BitBltFn> = unsafe { DYN_GDI32.get(b"BitBlt\0").unwrap() };
    pub static ref DYN_GET_DIBITS: libloading::Symbol<'static, GetDIBitsFn> = unsafe { DYN_GDI32.get(b"GetDIBits\0").unwrap() };
    pub static ref DYN_DELETE_OBJECT: libloading::Symbol<'static, DeleteObjectFn> = unsafe { DYN_GDI32.get(b"DeleteObject\0").unwrap() };
    pub static ref DYN_DELETE_DC: libloading::Symbol<'static, DeleteDCFn> = unsafe { DYN_GDI32.get(b"DeleteDC\0").unwrap() };

    pub static ref DYN_GET_DC: libloading::Symbol<'static, GetDCFn> = unsafe { DYN_USER32_SC.get(b"GetDC\0").unwrap() };
    pub static ref DYN_RELEASE_DC: libloading::Symbol<'static, ReleaseDCFn> = unsafe { DYN_USER32_SC.get(b"ReleaseDC\0").unwrap() };
    pub static ref DYN_GET_SYSTEM_METRICS: libloading::Symbol<'static, GetSystemMetricsFn> = unsafe { DYN_USER32_SC.get(b"GetSystemMetrics\0").unwrap() };
}

pub fn get_screen_size() -> (u32, u32) {
    unsafe {
        let w = DYN_GET_SYSTEM_METRICS(SM_CXSCREEN);
        let h = DYN_GET_SYSTEM_METRICS(SM_CYSCREEN);
        (w as u32, h as u32)
    }
}

pub fn capture_region(x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let w = width as i32;
    let h = height as i32;

    unsafe {
        let hdc_screen = DYN_GET_DC(ptr::null_mut());
        if hdc_screen.is_null() {
            return Err("getdc fail".into());
        }

        let hdc_mem = DYN_CREATE_COMPATIBLE_DC(hdc_screen);
        if hdc_mem.is_null() {
            DYN_RELEASE_DC(ptr::null_mut(), hdc_screen);
            return Err("createcompatibledc fail".into());
        }

        let hbm = DYN_CREATE_COMPATIBLE_BITMAP(hdc_screen, w, h);
        if hbm.is_null() {
            DYN_DELETE_DC(hdc_mem);
            DYN_RELEASE_DC(ptr::null_mut(), hdc_screen);
            return Err("createcompatiblebitmap fail".into());
        }

        let old_bm = DYN_SELECT_OBJECT(hdc_mem, hbm);
        let result = DYN_BIT_BLT(hdc_mem, 0, 0, w, h, hdc_screen, x, y, SRCCOPY);
        if result == 0 {
            DYN_SELECT_OBJECT(hdc_mem, old_bm);
            DYN_DELETE_OBJECT(hbm);
            DYN_DELETE_DC(hdc_mem);
            DYN_RELEASE_DC(ptr::null_mut(), hdc_screen);
            return Err("bitblt failed".into());
        }

        let mut bmi = BitmapInfo {
            bmi_header: BitmapInfoHeader {
                bi_size: std::mem::size_of::<BitmapInfoHeader>() as u32,
                bi_width: w,
                bi_height: -h, 
                bi_planes: 1,
                bi_bit_count: 32, 
                bi_compression: BI_RGB,
                bi_size_image: 0,
                bi_x_pels_per_meter: 0,
                bi_y_pels_per_meter: 0,
                bi_clr_used: 0,
                bi_clr_important: 0,
            },
            bmi_colors: [0],
        };

        let buf_size = (w * h * 4) as usize;
        let mut bgra_buf: Vec<u8> = vec![0u8; buf_size];

        let lines = DYN_GET_DIBITS(
            hdc_mem,
            hbm,
            0,
            h as u32,
            bgra_buf.as_mut_ptr(),
            &mut bmi,
            DIB_RGB_COLORS,
        );

        DYN_SELECT_OBJECT(hdc_mem, old_bm);
        DYN_DELETE_OBJECT(hbm);
        DYN_DELETE_DC(hdc_mem);
        DYN_RELEASE_DC(ptr::null_mut(), hdc_screen);

        if lines == 0 {
            return Err("Getdibits fail".into());
        }

        for i in (0..buf_size).step_by(4) {
            bgra_buf.swap(i, i + 2); 
        }

        Ok(bgra_buf)
    }
}

/*
// fallback to screenshots
pub fn capture_region_screenshots(x: i32, y: i32, width: u32, height: u32) -> Result<Vec<u8>, String> {
    use screenshots::Screen;

    let screens = Screen::all().map_err(|e| format!("Screen::all failed: {}", e))?;
    let primary = screens.into_iter().next().ok_or("No screen found")?;

    let capture = primary
        .capture_area(x, y, width, height)
        .map_err(|e| format!("capture_area failed: {}", e))?;

    let rgba_buf: Vec<u8> = capture.into_raw();
    Ok(rgba_buf)
}
*/