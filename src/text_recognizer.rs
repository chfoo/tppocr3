use std::ffi::{CStr, CString};

use anyhow::bail;
use tesseract_sys::TessBaseAPI;

pub struct TextRecognizer {
    api: *mut TessBaseAPI,
}

impl TextRecognizer {
    pub fn new(data_path: &str, language: &str) -> anyhow::Result<Self> {
        let api = unsafe {
            let api = tesseract_sys::TessBaseAPICreate();
            let c_data_path = CString::new(data_path)?;
            let c_language = CString::new(language)?;

            let result = tesseract_sys::TessBaseAPIInit2(
                api,
                c_data_path.as_ptr(),
                c_language.as_ptr(),
                tesseract_sys::TessOcrEngineMode_OEM_LSTM_ONLY,
            );

            if result != 0 {
                bail!("tesseract initialization returned error code {}", result);
            }

            tesseract_sys::TessBaseAPISetPageSegMode(
                api,
                tesseract_sys::TessPageSegMode_PSM_SINGLE_BLOCK,
            );

            api
        };

        Ok(Self { api })
    }

    pub fn set_image(&self, data: &[u32], width: u32, height: u32) {
        unsafe {
            tesseract_sys::TessBaseAPISetImage(
                self.api,
                data.as_ptr() as *const u8,
                width as i32,
                height as i32,
                4,
                (width * 4) as i32,
            );
            // TODO: Allow config DPI
            tesseract_sys::TessBaseAPISetSourceResolution(self.api, 300);
        }
    }

    pub fn set_rectangle(&self, left: u32, top: u32, width: u32, height: u32) {
        unsafe {
            tesseract_sys::TessBaseAPISetRectangle(
                self.api,
                left as i32,
                top as i32,
                width as i32,
                height as i32,
            )
        }
    }

    pub fn recognize(&self) -> anyhow::Result<()> {
        let result = unsafe { tesseract_sys::TessBaseAPIRecognize(self.api, std::ptr::null_mut()) };

        if result != 0 {
            bail!("tesseract recognize error code {}", result);
        } else {
            Ok(())
        }
    }

    pub fn get_text(&self) -> String {
        unsafe {
            let raw_c_string = tesseract_sys::TessBaseAPIGetUTF8Text(self.api);
            let c_string = CStr::from_ptr(raw_c_string);
            let string = c_string.to_string_lossy().to_string();
            tesseract_sys::TessDeleteText(raw_c_string);

            string
        }
    }

    fn get_boxes(&self, level: tesseract_sys::TessPageIteratorLevel) -> Vec<BoundingBox> {
        let mut boxes = Vec::new();

        unsafe {
            let iterator = tesseract_sys::TessBaseAPIGetIterator(self.api);
            let page_iterator = tesseract_sys::TessResultIteratorGetPageIterator(iterator);

            if !iterator.is_null() {
                loop {
                    let confidence = tesseract_sys::TessResultIteratorConfidence(iterator, level);
                    let mut x1 = 0;
                    let mut y1 = 0;
                    let mut x2 = 0;
                    let mut y2 = 0;
                    tesseract_sys::TessPageIteratorBoundingBox(
                        page_iterator,
                        level,
                        &mut x1,
                        &mut y1,
                        &mut x2,
                        &mut y2,
                    );

                    boxes.push(BoundingBox {
                        confidence: confidence / 100.0,
                        x1,
                        y1,
                        x2,
                        y2,
                    });

                    if tesseract_sys::TessResultIteratorNext(iterator, level) == 0 {
                        break;
                    }
                }
            }
        }

        boxes
    }

    pub fn get_block_boxes(&self) -> Vec<BoundingBox> {
        self.get_boxes(tesseract_sys::TessPageIteratorLevel_RIL_BLOCK)
    }

    pub fn get_word_boxes(&self) -> Vec<BoundingBox> {
        self.get_boxes(tesseract_sys::TessPageIteratorLevel_RIL_WORD)
    }
}

impl Drop for TextRecognizer {
    fn drop(&mut self) {
        unsafe {
            tesseract_sys::TessBaseAPIDelete(self.api);
        }
    }
}

pub struct BoundingBox {
    pub confidence: f32, // in range [0.0, 1.0] where 1.0 is 100% confidence
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}
