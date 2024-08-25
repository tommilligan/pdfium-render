//! Defines the [PdfiumLibraryBindings] trait, containing run-time bindings to the FPDF_*
//! functions exported by the Pdfium library.
//!
//! By default, `pdfium-render` attempts to bind against the latest released version
//! of the Pdfium API. To explicitly bind against an older version, select one of the
//! crate's Pdfium version feature flags when taking `pdfium-render` as a dependency
//! in your project's `Cargo.toml`.
//!
//! Doc comments on functions in this trait are taken directly from the Pdfium header files
//! and as such are copyright by the Pdfium authors and Google. They are reproduced here
//! as a courtesy for API consumers. The original comments can be found in the Pdfium repository at:
//! <https://pdfium.googlesource.com/pdfium/+/refs/heads/main/public/>

// Include the appropriate implementation of the PdfiumLibraryBindings trait for the
// target architecture and threading model.

// Conditional compilation is used to compile different implementations of
// the PdfiumLibraryBindings trait depending on whether we are compiling to a WASM module,
// a native shared library, or a statically linked library.

#[cfg(not(target_arch = "wasm32"))]
#[cfg(not(feature = "static"))]
pub(crate) mod dynamic;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(feature = "static")]
pub(crate) mod static_bindings;

#[cfg(target_arch = "wasm32")]
pub(crate) mod wasm;

// These implementations are all single-threaded (because Pdfium itself is single-threaded).
// Any of them can be wrapped by thread_safe::ThreadSafePdfiumBindings to
// create a thread-safe architecture-specific implementation of the PdfiumLibraryBindings trait.

#[cfg(feature = "thread_safe")]
pub(crate) mod thread_safe;

pub mod version;

use crate::bindgen::{
    size_t, FPDF_CharsetFontMap, FPDFANNOT_COLORTYPE, FPDF_ACTION, FPDF_ANNOTATION,
    FPDF_ANNOTATION_SUBTYPE, FPDF_ANNOT_APPEARANCEMODE, FPDF_ATTACHMENT, FPDF_AVAIL, FPDF_BITMAP,
    FPDF_BOOKMARK, FPDF_BOOL, FPDF_CLIPPATH, FPDF_COLORSCHEME, FPDF_DEST, FPDF_DOCUMENT,
    FPDF_DUPLEXTYPE, FPDF_DWORD, FPDF_FILEACCESS, FPDF_FILEIDTYPE, FPDF_FILEWRITE, FPDF_FONT,
    FPDF_FORMFILLINFO, FPDF_FORMHANDLE, FPDF_GLYPHPATH, FPDF_IMAGEOBJ_METADATA,
    FPDF_JAVASCRIPT_ACTION, FPDF_LIBRARY_CONFIG, FPDF_LINK, FPDF_OBJECT_TYPE, FPDF_PAGE,
    FPDF_PAGELINK, FPDF_PAGEOBJECT, FPDF_PAGEOBJECTMARK, FPDF_PAGERANGE, FPDF_PATHSEGMENT,
    FPDF_SCHHANDLE, FPDF_SIGNATURE, FPDF_STRUCTELEMENT, FPDF_STRUCTELEMENT_ATTR, FPDF_STRUCTTREE,
    FPDF_SYSFONTINFO, FPDF_TEXTPAGE, FPDF_TEXT_RENDERMODE, FPDF_WCHAR, FPDF_WIDESTRING,
    FPDF_XOBJECT, FS_FLOAT, FS_MATRIX, FS_POINTF, FS_QUADPOINTSF, FS_RECTF, FS_SIZEF,
    FX_DOWNLOADHINTS, FX_FILEAVAIL, IFSDK_PAUSE,
};

#[cfg(any(
    feature = "pdfium_6490",
    feature = "pdfium_6555",
    feature = "pdfium_6569",
    feature = "pdfium_6611",
    feature = "pdfium_6666",
    feature = "pdfium_future"
))]
use crate::bindgen::FPDF_STRUCTELEMENT_ATTR_VALUE;

#[cfg(feature = "pdfium_use_skia")]
use crate::bindgen::FPDF_SKIA_CANVAS;

#[cfg(feature = "pdfium_enable_xfa")]
use crate::bindgen::{FPDF_BSTR, FPDF_RESULT};

use crate::bindings::version::PdfiumApiVersion;
use crate::error::{PdfiumError, PdfiumInternalError};
use crate::pdf::color::PdfColor;
use crate::pdf::document::page::object::private::internal::PdfPageObjectPrivate;
use crate::pdf::document::page::object::PdfPageObject;
use crate::pdf::document::page::PdfPage;
use crate::pdf::document::PdfDocument;
use crate::utils::pixels::{
    bgra_to_rgba, rgba_to_bgra, unaligned_bgr_to_rgba, unaligned_rgb_to_bgra,
};
use crate::utils::utf16le::{
    get_pdfium_utf16le_bytes_from_str, get_string_from_pdfium_utf16le_bytes,
};
use std::os::raw::{
    c_char, c_double, c_float, c_int, c_long, c_uchar, c_uint, c_ulong, c_ushort, c_void,
};

/// Platform-independent function bindings to an external Pdfium library.
/// On most platforms this will be an external shared library loaded dynamically
/// at runtime, either bundled alongside your compiled Rust application or provided as a system
/// library by the platform. On WASM, this will be a set of Javascript functions exposed by a
/// separate WASM module that is imported into the same browser context.
///
/// Pdfium's API uses three different string types: classic C-style null-terminated char arrays,
/// UTF-8 byte arrays, and a UTF-16LE byte array type named `FPDF_WIDESTRING`. For functions that take a
/// C-style string or a UTF-8 byte array, `pdfium-render`'s binding will take the standard Rust `&str` type.
/// For functions that take an `FPDF_WIDESTRING`, `pdfium-render` exposes two functions: the vanilla
/// `FPDF_*()` function that takes an `FPDF_WIDESTRING`, and an additional `FPDF_*_str()` helper function
/// that takes a standard Rust `&str` and converts it internally to an `FPDF_WIDESTRING` before calling
/// Pdfium. Examples of functions with additional `_str()` helpers include `FPDFBookmark_Find()`,
/// `FPDFAnnot_SetStringValue()`, and `FPDFText_SetText()`.
///
/// The [PdfiumLibraryBindings::get_pdfium_utf16le_bytes_from_str] and
/// [PdfiumLibraryBindings::get_string_from_pdfium_utf16le_bytes] functions are provided
/// for converting to and from UTF-16LE in your own code.
///
/// The following Pdfium functions have different signatures in this trait compared to their
/// native function signatures in Pdfium:
/// * [PdfiumLibraryBindings::FPDF_LoadDocument]: this function is not available when compiling to WASM.
/// * [PdfiumLibraryBindings::FPDFBitmap_GetBuffer]: the return type of this function is modified
///   when compiling to WASM. Instead of returning `*mut c_void`, it returns `*const c_void`.
///   This is to encourage callers to avoid directly mutating the returned buffer, as this is not
///   supported when compiling to WASM. Instead, callers should use the provided
///   [PdfiumLibraryBindings::FPDFBitmap_SetBuffer] convenience function to apply modified pixel data
///   to a bitmap.
pub trait PdfiumLibraryBindings {
    /// Returns the canonical C-style boolean integer value 1, indicating `true`.
    #[inline]
    #[allow(non_snake_case)]
    fn TRUE(&self) -> FPDF_BOOL {
        1
    }

    /// Returns the canonical C-style boolean integer value 0, indicating `false`.
    #[inline]
    #[allow(non_snake_case)]
    fn FALSE(&self) -> FPDF_BOOL {
        0
    }

    /// Converts from a C-style boolean integer to a Rust `bool`.
    ///
    /// Assumes `PdfiumLibraryBindings::FALSE()` indicates `false` and any other value indicates `true`.
    #[inline]
    fn is_true(&self, bool: FPDF_BOOL) -> bool {
        bool != self.FALSE()
    }

    /// Converts the given Rust `bool` into a Pdfium `FPDF_BOOL`.
    #[inline]
    fn bool_to_pdfium(&self, bool: bool) -> FPDF_BOOL {
        if bool {
            self.TRUE()
        } else {
            self.FALSE()
        }
    }

    /// Converts from a C-style boolean integer to a Rust `Result`.
    ///
    /// Assumes `PdfiumLibraryBindings::FALSE()` indicates `false` and any other value indicates `true`.
    ///
    /// A value of `PdfiumLibraryBindings::FALSE()` will return a [PdfiumInternalError::Unknown].
    /// All other values will return `Ok(())`.
    #[inline]
    fn to_result(&self, bool: FPDF_BOOL) -> Result<(), PdfiumError> {
        if self.is_true(bool) {
            Ok(())
        } else {
            Err(PdfiumError::PdfiumLibraryInternalError(
                PdfiumInternalError::Unknown,
            ))
        }
    }

    /// Converts the given Rust `&str` into an UTF16-LE encoded byte buffer.
    #[inline]
    fn get_pdfium_utf16le_bytes_from_str(&self, str: &str) -> Vec<u8> {
        get_pdfium_utf16le_bytes_from_str(str)
    }

    /// Converts the bytes in the given buffer from UTF16-LE to a standard Rust `String`.
    #[inline]
    #[allow(unused_mut)] // The buffer must be mutable when compiling to WASM.
    fn get_string_from_pdfium_utf16le_bytes(&self, mut buffer: Vec<u8>) -> Option<String> {
        get_string_from_pdfium_utf16le_bytes(buffer)
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel BGR,
    /// into pixel data encoded as four-channel RGBA. A new alpha channel is created with full opacity.
    #[inline]
    fn bgr_to_rgba(&self, bgr: &[u8]) -> Vec<u8> {
        unaligned_bgr_to_rgba(bgr)
    }

    /// Converts the given byte array, containing pixel data encoded as four-channel BGRA,
    /// into pixel data encoded as four-channel RGBA.
    #[inline]
    fn bgra_to_rgba(&self, bgra: &[u8]) -> Vec<u8> {
        bgra_to_rgba(bgra)
    }

    /// Converts the given byte array, containing pixel data encoded as three-channel RGB,
    /// into pixel data encoded as four-channel BGRA. A new alpha channel is created with full opacity.
    #[inline]
    fn rgb_to_bgra(&self, rgb: &[u8]) -> Vec<u8> {
        unaligned_rgb_to_bgra(rgb)
    }

    /// Converts the given byte array, containing pixel data encoded as four-channel RGBA,
    /// into pixel data encoded as four-channel BGRA.
    #[inline]
    fn rgba_to_bgra(&self, rgba: &[u8]) -> Vec<u8> {
        rgba_to_bgra(rgba)
    }

    /// Returns Pdfium's internal `FPDF_DOCUMENT` handle for the given [PdfDocument].
    #[inline]
    fn get_handle_from_document(&self, document: &PdfDocument) -> FPDF_DOCUMENT {
        document.handle()
    }

    /// Returns Pdfium's internal `FPDF_PAGE` handle for the given [PdfPage].
    #[inline]
    fn get_handle_from_page(&self, page: &PdfPage) -> FPDF_PAGE {
        page.page_handle()
    }

    /// Returns Pdfium's internal `FPDF_PAGEOBJECT` handle for the given [PdfPageObject].
    #[inline]
    fn get_handle_from_object(&self, object: &PdfPageObject) -> FPDF_PAGEOBJECT {
        object.get_object_handle()
    }

    /// Returns the API version of the Pdfium FPDF_* API currently in use.
    ///
    /// By default, `pdfium-render` attempts to bind against the latest released version
    /// of the Pdfium API. To explicitly bind against an older version, select one of the
    /// crate's Pdfium version feature flags when taking `pdfium-render` as a dependency
    /// in your project's `Cargo.toml`.
    #[inline]
    fn version(&self) -> PdfiumApiVersion {
        PdfiumApiVersion::current()
    }

    #[doc = " Function: FPDF_InitLibraryWithConfig\n          Initialize the PDFium library and allocate global resources for it.\n Parameters:\n          config - configuration information as above.\n Return value:\n          None.\n Comments:\n          You have to call this function before you can call any PDF\n          processing functions."]
    #[allow(non_snake_case)]
    fn FPDF_InitLibraryWithConfig(&self, config: *const FPDF_LIBRARY_CONFIG);

    #[doc = " Function: FPDF_InitLibrary\n          Initialize the PDFium library (alternative form).\n Parameters:\n          None\n Return value:\n          None.\n Comments:\n          Convenience function to call FPDF_InitLibraryWithConfig() with a\n          default configuration for backwards compatibility purposes. New\n          code should call FPDF_InitLibraryWithConfig() instead. This will\n          be deprecated in the future."]
    #[allow(non_snake_case)]
    fn FPDF_InitLibrary(&self);

    #[doc = " Function: FPDF_DestroyLibrary\n          Release global resources allocated to the PDFium library by\n          FPDF_InitLibrary() or FPDF_InitLibraryWithConfig().\n Parameters:\n          None.\n Return value:\n          None.\n Comments:\n          After this function is called, you must not call any PDF\n          processing functions.\n\n          Calling this function does not automatically close other\n          objects. It is recommended to close other objects before\n          closing the library with this function."]
    #[allow(non_snake_case)]
    fn FPDF_DestroyLibrary(&self);

    #[doc = " Function: FPDF_SetSandBoxPolicy\n          Set the policy for the sandbox environment.\n Parameters:\n          policy -   The specified policy for setting, for example:\n                     FPDF_POLICY_MACHINETIME_ACCESS.\n          enable -   True to enable, false to disable the policy.\n Return value:\n          None."]
    #[allow(non_snake_case)]
    fn FPDF_SetSandBoxPolicy(&self, policy: FPDF_DWORD, enable: FPDF_BOOL);

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "pdfium_use_win32")]
    /// Sets printing mode when printing on Windows.
    ///
    ///    mode - FPDF_PRINTMODE_EMF to output EMF (default)
    ///
    ///           FPDF_PRINTMODE_TEXTONLY to output text only (for charstream devices)
    ///
    ///           FPDF_PRINTMODE_POSTSCRIPT2 to output level 2 PostScript into
    ///           EMF as a series of GDI comments.
    ///
    ///           FPDF_PRINTMODE_POSTSCRIPT3 to output level 3 PostScript into
    ///           EMF as a series of GDI comments.
    ///
    ///           FPDF_PRINTMODE_POSTSCRIPT2_PASSTHROUGH to output level 2
    ///           PostScript via ExtEscape() in PASSTHROUGH mode.
    ///
    ///           FPDF_PRINTMODE_POSTSCRIPT3_PASSTHROUGH to output level 3
    ///           PostScript via ExtEscape() in PASSTHROUGH mode.
    ///
    ///           FPDF_PRINTMODE_EMF_IMAGE_MASKS to output EMF, with more
    ///           efficient processing of documents containing image masks.
    ///
    ///           FPDF_PRINTMODE_POSTSCRIPT3_TYPE42 to output level 3
    ///           PostScript with embedded Type 42 fonts, when applicable, into
    ///           EMF as a series of GDI comments.
    ///
    ///           FPDF_PRINTMODE_POSTSCRIPT3_TYPE42_PASSTHROUGH to output level
    ///           3 PostScript with embedded Type 42 fonts, when applicable,
    ///           via ExtEscape() in PASSTHROUGH mode.
    ///
    /// Returns `true` if successful, `false` if unsuccessful (typically invalid input).
    #[allow(non_snake_case)]
    fn FPDF_SetPrintMode(&self, mode: c_int);

    #[allow(non_snake_case)]
    fn FPDF_GetLastError(&self) -> c_ulong;

    #[allow(non_snake_case)]
    fn FPDF_ARGB(&self, a: u8, r: u8, g: u8, b: u8) -> FPDF_DWORD {
        PdfColor::new(r, g, b, a).as_pdfium_color()
    }

    #[allow(non_snake_case)]
    fn FPDF_GetBValue(&self, argb: FPDF_DWORD) -> u8 {
        PdfColor::from_pdfium(argb).blue()
    }

    #[allow(non_snake_case)]
    fn FPDF_GetGValue(&self, argb: FPDF_DWORD) -> u8 {
        PdfColor::from_pdfium(argb).green()
    }

    #[allow(non_snake_case)]
    fn FPDF_GetRValue(&self, argb: FPDF_DWORD) -> u8 {
        PdfColor::from_pdfium(argb).red()
    }

    #[allow(non_snake_case)]
    fn FPDF_GetAValue(&self, argb: FPDF_DWORD) -> u8 {
        PdfColor::from_pdfium(argb).alpha()
    }

    #[allow(non_snake_case)]
    fn FPDF_CreateNewDocument(&self) -> FPDF_DOCUMENT;

    /// This function is not available when compiling to WASM. You must use one of the
    /// [PdfiumLibraryBindings::FPDF_LoadMemDocument()], [PdfiumLibraryBindings::FPDF_LoadMemDocument64()],
    /// or [PdfiumLibraryBindings::FPDF_LoadCustomDocument()] functions instead.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(non_snake_case)]
    fn FPDF_LoadDocument(&self, file_path: &str, password: Option<&str>) -> FPDF_DOCUMENT;

    /// Note that all calls to [PdfiumLibraryBindings::FPDF_LoadMemDocument()] are
    /// internally upgraded to [PdfiumLibraryBindings::FPDF_LoadMemDocument64()].
    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_LoadMemDocument(&self, bytes: &[u8], password: Option<&str>) -> FPDF_DOCUMENT {
        self.FPDF_LoadMemDocument64(bytes, password)
    }

    #[allow(non_snake_case)]
    fn FPDF_LoadMemDocument64(&self, data_buf: &[u8], password: Option<&str>) -> FPDF_DOCUMENT;

    #[allow(non_snake_case)]
    fn FPDF_LoadCustomDocument(
        &self,
        pFileAccess: *mut FPDF_FILEACCESS,
        password: Option<&str>,
    ) -> FPDF_DOCUMENT;

    #[allow(non_snake_case)]
    fn FPDF_SaveAsCopy(
        &self,
        document: FPDF_DOCUMENT,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: FPDF_DWORD,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDF_SaveWithVersion(
        &self,
        document: FPDF_DOCUMENT,
        pFileWrite: *mut FPDF_FILEWRITE,
        flags: FPDF_DWORD,
        fileVersion: c_int,
    ) -> FPDF_BOOL;

    /// Create a document availability provider.
    ///
    ///   `file_avail` - pointer to file availability interface.
    ///
    ///   `file`       - pointer to a file access interface.
    ///
    /// Returns a handle to the document availability provider, or `NULL` on error.
    ///
    /// [PdfiumLibraryBindings::FPDFAvail_Destroy] must be called when done with the
    /// availability provider.
    #[allow(non_snake_case)]
    fn FPDFAvail_Create(
        &self,
        file_avail: *mut FX_FILEAVAIL,
        file: *mut FPDF_FILEACCESS,
    ) -> FPDF_AVAIL;

    /// Destroy the `avail` document availability provider.
    ///
    ///   `avail` - handle to document availability provider to be destroyed.
    #[allow(non_snake_case)]
    fn FPDFAvail_Destroy(&self, avail: FPDF_AVAIL);

    /// Checks if the document is ready for loading; if not, gets download hints.
    ///
    ///   `avail` - handle to document availability provider.
    ///
    ///   `hints` - pointer to a download hints interface.
    ///
    /// Returns one of:
    ///
    ///   `PDF_DATA_ERROR`: A common error is returned. Data availability unknown.
    ///
    ///   `PDF_DATA_NOTAVAIL`: Data not yet available.
    ///
    ///   `PDF_DATA_AVAIL`: Data available.
    ///
    /// Applications should call this function whenever new data arrives, and process
    /// all the generated download hints, if any, until the function returns
    /// `PDF_DATA_ERROR` or `PDF_DATA_AVAIL`.
    ///
    /// If `hints` is `NULL`, the function just checks current document availability.
    ///
    /// Once all data is available, call [PdfiumLibraryBindings::FPDFAvail_GetDocument] to get
    /// a document handle.
    #[allow(non_snake_case)]
    fn FPDFAvail_IsDocAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int;

    /// Get document from the availability provider.
    ///
    ///   `avail`    - handle to document availability provider.
    ///
    ///   `password` - password for decrypting the PDF file. Optional.
    ///
    /// Returns a handle to the document.
    ///
    /// When [PdfiumLibraryBindings::FPDFAvail_IsDocAvail] returns `TRUE`, call
    /// [PdfiumLibraryBindings::FPDFAvail_GetDocument] to\n retrieve the document handle.
    /// See the comments for [PdfiumLibraryBindings::FPDF_LoadDocument] regarding the encoding
    /// for `password`.
    #[allow(non_snake_case)]
    fn FPDFAvail_GetDocument(&self, avail: FPDF_AVAIL, password: Option<&str>) -> FPDF_DOCUMENT;

    /// Get the page number for the first available page in a linearized PDF.
    ///
    ///   `doc` - document handle.
    ///
    /// Returns the zero-based index for the first available page.
    ///
    /// For most linearized PDFs, the first available page will be the first page,
    /// however, some PDFs might make another page the first available page.
    ///
    /// For non-linearized PDFs, this function will always return zero.
    #[allow(non_snake_case)]
    fn FPDFAvail_GetFirstPageNum(&self, doc: FPDF_DOCUMENT) -> c_int;

    /// Check if `page_index` is ready for loading, if not, get the `FX_DOWNLOADHINTS`.
    ///
    ///   `avail`      - handle to document availability provider.
    ///
    ///   `page_index` - index number of the page. Zero for the first page.
    ///
    ///   `hints`      - pointer to a download hints interface. Populated if
    ///                  `page_index` is not available.
    ///
    /// Returns one of:
    ///
    ///   `PDF_DATA_ERROR`: A common error is returned. Data availability unknown.
    ///
    ///   `PDF_DATA_NOTAVAIL`: Data not yet available.
    ///
    ///   `PDF_DATA_AVAIL`: Data available.
    ///
    /// This function can be called only after [PdfiumLibraryBindings::FPDFAvail_GetDocument]
    /// is called. Applications should call this function whenever new data arrives and process
    /// all the generated download `hints`, if any, until this function returns `PDF_DATA_ERROR`
    /// or `PDF_DATA_AVAIL`. Applications can then perform page loading.
    ///
    /// If `hints` is `NULL`, the function just check current availability of specified page.
    #[allow(non_snake_case)]
    fn FPDFAvail_IsPageAvail(
        &self,
        avail: FPDF_AVAIL,
        page_index: c_int,
        hints: *mut FX_DOWNLOADHINTS,
    ) -> c_int;

    /// Check if form data is ready for initialization; if not, get the `FX_DOWNLOADHINTS`.
    ///
    ///   `avail` - handle to document availability provider.
    ///
    ///   `hints` - pointer to a download hints interface. Populated if form is not
    ///             ready for initialization.
    ///
    /// Returns one of:
    ///
    ///   `PDF_FORM_ERROR`: A common error, in general incorrect parameters.
    ///
    ///   `PDF_FORM_NOTAVAIL`: Data not available.
    ///
    ///   `PDF_FORM_AVAIL`: Data available.
    ///
    ///   `PDF_FORM_NOTEXIST`: No form data.
    ///
    /// This function can be called only after [PdfiumLibraryBindings::FPDFAvail_GetDocument]
    /// is called. The application should call this function whenever new data arrives and
    /// process all the generated download `hints`, if any, until the function returns
    /// `PDF_FORM_ERROR`, `PDF_FORM_AVAIL` or `PDF_FORM_NOTEXIST`.
    ///
    /// If `hints` is `NULL`, the function just check current form availability.
    ///
    /// Applications can then perform page loading. It is recommend to call
    /// [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment] when `PDF_FORM_AVAIL` is returned.
    #[allow(non_snake_case)]
    fn FPDFAvail_IsFormAvail(&self, avail: FPDF_AVAIL, hints: *mut FX_DOWNLOADHINTS) -> c_int;

    /// Check whether a document is a linearized PDF.
    ///
    ///   `avail` - handle to document availability provider.
    ///
    /// Returns one of:
    ///
    ///   `PDF_LINEARIZED`
    ///
    ///   `PDF_NOT_LINEARIZED`
    ///
    ///   `PDF_LINEARIZATION_UNKNOWN`
    ///
    /// [PdfiumLibraryBindings::FPDFAvail_IsLinearized] will return `PDF_LINEARIZED` or
    /// `PDF_NOT_LINEARIZED` once we have received 1kb of data. If the file's size is less
    /// than 1kb, it returns `PDF_LINEARIZATION_UNKNOWN` as there is insufficient information
    // to determine if the PDF is linearlized.
    #[allow(non_snake_case)]
    fn FPDFAvail_IsLinearized(&self, avail: FPDF_AVAIL) -> c_int;

    #[doc = " Function: FPDF_ClosePage\n          Close a loaded PDF page.\n Parameters:\n          page        -   Handle to the loaded page.\n Return value:\n          None."]
    #[allow(non_snake_case)]
    fn FPDF_ClosePage(&self, page: FPDF_PAGE);

    #[doc = " Function: FPDF_CloseDocument\n          Close a loaded PDF document.\n Parameters:\n          document    -   Handle to the loaded document.\n Return value:\n          None."]
    #[allow(non_snake_case)]
    fn FPDF_CloseDocument(&self, document: FPDF_DOCUMENT);

    #[doc = " Function: FPDF_DeviceToPage\n          Convert the screen coordinates of a point to page coordinates.\n Parameters:\n          page        -   Handle to the page. Returned by FPDF_LoadPage.\n          start_x     -   Left pixel position of the display area in\n                          device coordinates.\n          start_y     -   Top pixel position of the display area in device\n                          coordinates.\n          size_x      -   Horizontal size (in pixels) for displaying the page.\n          size_y      -   Vertical size (in pixels) for displaying the page.\n          rotate      -   Page orientation:\n                            0 (normal)\n                            1 (rotated 90 degrees clockwise)\n                            2 (rotated 180 degrees)\n                            3 (rotated 90 degrees counter-clockwise)\n          device_x    -   X value in device coordinates to be converted.\n          device_y    -   Y value in device coordinates to be converted.\n          page_x      -   A pointer to a double receiving the converted X\n                          value in page coordinates.\n          page_y      -   A pointer to a double receiving the converted Y\n                          value in page coordinates.\n Return value:\n          Returns true if the conversion succeeds, and |page_x| and |page_y|\n          successfully receives the converted coordinates.\n Comments:\n          The page coordinate system has its origin at the left-bottom corner\n          of the page, with the X-axis on the bottom going to the right, and\n          the Y-axis on the left side going up.\n\n          NOTE: this coordinate system can be altered when you zoom, scroll,\n          or rotate a page, however, a point on the page should always have\n          the same coordinate values in the page coordinate system.\n\n          The device coordinate system is device dependent. For screen device,\n          its origin is at the left-top corner of the window. However this\n          origin can be altered by the Windows coordinate transformation\n          utilities.\n\n          You must make sure the start_x, start_y, size_x, size_y\n          and rotate parameters have exactly same values as you used in\n          the FPDF_RenderPage() function call."]
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDF_DeviceToPage(
        &self,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        device_x: c_int,
        device_y: c_int,
        page_x: *mut c_double,
        page_y: *mut c_double,
    ) -> FPDF_BOOL;

    #[doc = " Function: FPDF_PageToDevice\n          Convert the page coordinates of a point to screen coordinates.\n Parameters:\n          page        -   Handle to the page. Returned by FPDF_LoadPage.\n          start_x     -   Left pixel position of the display area in\n                          device coordinates.\n          start_y     -   Top pixel position of the display area in device\n                          coordinates.\n          size_x      -   Horizontal size (in pixels) for displaying the page.\n          size_y      -   Vertical size (in pixels) for displaying the page.\n          rotate      -   Page orientation:\n                            0 (normal)\n                            1 (rotated 90 degrees clockwise)\n                            2 (rotated 180 degrees)\n                            3 (rotated 90 degrees counter-clockwise)\n          page_x      -   X value in page coordinates.\n          page_y      -   Y value in page coordinate.\n          device_x    -   A pointer to an integer receiving the result X\n                          value in device coordinates.\n          device_y    -   A pointer to an integer receiving the result Y\n                          value in device coordinates.\n Return value:\n          Returns true if the conversion succeeds, and |device_x| and\n          |device_y| successfully receives the converted coordinates.\n Comments:\n          See comments for FPDF_DeviceToPage()."]
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDF_PageToDevice(
        &self,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        page_x: c_double,
        page_y: c_double,
        device_x: *mut c_int,
        device_y: *mut c_int,
    ) -> FPDF_BOOL;

    #[doc = " Function: FPDF_GetFileVersion\n          Get the file version of the given PDF document.\n Parameters:\n          doc         -   Handle to a document.\n          fileVersion -   The PDF file version. File version: 14 for 1.4, 15\n                          for 1.5, ...\n Return value:\n          True if succeeds, false otherwise.\n Comments:\n          If the document was created by FPDF_CreateNewDocument,\n          then this function will always fail."]
    #[allow(non_snake_case)]
    fn FPDF_GetFileVersion(&self, doc: FPDF_DOCUMENT, fileVersion: *mut c_int) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FPDF_DocumentHasValidCrossReferenceTable\n          Whether the document's cross reference table is valid or not.\n Parameters:\n          document    -   Handle to a document. Returned by FPDF_LoadDocument.\n Return value:\n          True if the PDF parser did not encounter problems parsing the cross\n          reference table. False if the parser could not parse the cross\n          reference table and the table had to be rebuild from other data\n          within the document.\n Comments:\n          The return value can change over time as the PDF parser evolves."]
    #[allow(non_snake_case)]
    fn FPDF_DocumentHasValidCrossReferenceTable(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FPDF_GetTrailerEnds\n          Get the byte offsets of trailer ends.\n Parameters:\n          document    -   Handle to document. Returned by FPDF_LoadDocument().\n          buffer      -   The address of a buffer that receives the\n                          byte offsets.\n          length      -   The size, in ints, of |buffer|.\n Return value:\n          Returns the number of ints in the buffer on success, 0 on error.\n\n |buffer| is an array of integers that describes the exact byte offsets of the\n trailer ends in the document. If |length| is less than the returned length,\n or |document| or |buffer| is NULL, |buffer| will not be modified."]
    #[allow(non_snake_case)]
    fn FPDF_GetTrailerEnds(
        &self,
        document: FPDF_DOCUMENT,
        buffer: *mut c_uint,
        length: c_ulong,
    ) -> c_ulong;

    #[doc = " Function: FPDF_GetDocPermissions\n          Get file permission flags of the document.\n Parameters:\n          document    -   Handle to a document. Returned by FPDF_LoadDocument.\n Return value:\n          A 32-bit integer indicating permission flags. Please refer to the\n          PDF Reference for detailed descriptions. If the document is not\n          protected or was unlocked by the owner, 0xffffffff will be returned."]
    #[allow(non_snake_case)]
    fn FPDF_GetDocPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong;

    #[cfg(any(
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406",
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[doc = " Function: FPDF_GetDocUserPermissions\n          Get user file permission flags of the document.\n Parameters:\n          document    -   Handle to a document. Returned by FPDF_LoadDocument.\n Return value:\n          A 32-bit integer indicating permission flags. Please refer to the\n          PDF Reference for detailed descriptions. If the document is not\n          protected, 0xffffffff will be returned. Always returns user\n          permissions, even if the document was unlocked by the owner."]
    #[allow(non_snake_case)]
    fn FPDF_GetDocUserPermissions(&self, document: FPDF_DOCUMENT) -> c_ulong;

    #[doc = " Function: FPDF_GetSecurityHandlerRevision\n          Get the revision for the security handler.\n Parameters:\n          document    -   Handle to a document. Returned by FPDF_LoadDocument.\n Return value:\n          The security handler revision number. Please refer to the PDF\n          Reference for a detailed description. If the document is not\n          protected, -1 will be returned."]
    #[allow(non_snake_case)]
    fn FPDF_GetSecurityHandlerRevision(&self, document: FPDF_DOCUMENT) -> c_int;

    #[doc = " Function: FPDF_GetPageCount\n          Get total number of pages in the document.\n Parameters:\n          document    -   Handle to document. Returned by FPDF_LoadDocument.\n Return value:\n          Total number of pages in the document."]
    #[allow(non_snake_case)]
    fn FPDF_GetPageCount(&self, document: FPDF_DOCUMENT) -> c_int;

    #[doc = " Function: FPDF_LoadPage\n          Load a page inside the document.\n Parameters:\n          document    -   Handle to document. Returned by FPDF_LoadDocument\n          page_index  -   Index number of the page. 0 for the first page.\n Return value:\n          A handle to the loaded page, or NULL if page load fails.\n Comments:\n          The loaded page can be rendered to devices using FPDF_RenderPage.\n          The loaded page can be closed using FPDF_ClosePage."]
    #[allow(non_snake_case)]
    fn FPDF_LoadPage(&self, document: FPDF_DOCUMENT, page_index: c_int) -> FPDF_PAGE;

    #[doc = " Experimental API.\n Function: FPDF_RenderPageBitmapWithColorScheme_Start\n          Start to render page contents to a device independent bitmap\n          progressively with a specified color scheme for the content.\n Parameters:\n          bitmap       -   Handle to the device independent bitmap (as the\n                           output buffer). Bitmap handle can be created by\n                           FPDFBitmap_Create function.\n          page         -   Handle to the page as returned by FPDF_LoadPage\n                           function.\n          start_x      -   Left pixel position of the display area in the\n                           bitmap coordinate.\n          start_y      -   Top pixel position of the display area in the\n                           bitmap coordinate.\n          size_x       -   Horizontal size (in pixels) for displaying the\n                           page.\n          size_y       -   Vertical size (in pixels) for displaying the page.\n          rotate       -   Page orientation: 0 (normal), 1 (rotated 90\n                           degrees clockwise), 2 (rotated 180 degrees),\n                           3 (rotated 90 degrees counter-clockwise).\n          flags        -   0 for normal display, or combination of flags\n                           defined in fpdfview.h. With FPDF_ANNOT flag, it\n                           renders all annotations that does not require\n                           user-interaction, which are all annotations except\n                           widget and popup annotations.\n          color_scheme -   Color scheme to be used in rendering the |page|.\n                           If null, this function will work similar to\n                           FPDF_RenderPageBitmap_Start().\n          pause        -   The IFSDK_PAUSE interface. A callback mechanism\n                           allowing the page rendering process.\n Return value:\n          Rendering Status. See flags for progressive process status for the\n          details."]
    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmapWithColorScheme_Start(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        color_scheme: *const FPDF_COLORSCHEME,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int;

    #[doc = " Function: FPDF_RenderPageBitmap_Start\n          Start to render page contents to a device independent bitmap\n          progressively.\n Parameters:\n          bitmap      -   Handle to the device independent bitmap (as the\n                          output buffer). Bitmap handle can be created by\n                          FPDFBitmap_Create().\n          page        -   Handle to the page, as returned by FPDF_LoadPage().\n          start_x     -   Left pixel position of the display area in the\n                          bitmap coordinates.\n          start_y     -   Top pixel position of the display area in the bitmap\n                          coordinates.\n          size_x      -   Horizontal size (in pixels) for displaying the page.\n          size_y      -   Vertical size (in pixels) for displaying the page.\n          rotate      -   Page orientation: 0 (normal), 1 (rotated 90 degrees\n                          clockwise), 2 (rotated 180 degrees), 3 (rotated 90\n                          degrees counter-clockwise).\n          flags       -   0 for normal display, or combination of flags\n                          defined in fpdfview.h. With FPDF_ANNOT flag, it\n                          renders all annotations that does not require\n                          user-interaction, which are all annotations except\n                          widget and popup annotations.\n          pause       -   The IFSDK_PAUSE interface.A callback mechanism\n                          allowing the page rendering process\n Return value:\n          Rendering Status. See flags for progressive process status for the\n          details."]
    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmap_Start(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
        pause: *mut IFSDK_PAUSE,
    ) -> c_int;

    #[doc = " Function: FPDF_RenderPage_Continue\n          Continue rendering a PDF page.\n Parameters:\n          page        -   Handle to the page, as returned by FPDF_LoadPage().\n          pause       -   The IFSDK_PAUSE interface (a callback mechanism\n                          allowing the page rendering process to be paused\n                          before it's finished). This can be NULL if you\n                          don't want to pause.\n Return value:\n          The rendering status. See flags for progressive process status for\n          the details."]
    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Continue(&self, page: FPDF_PAGE, pause: *mut IFSDK_PAUSE) -> c_int;

    #[doc = " Function: FPDF_RenderPage_Close\n          Release the resource allocate during page rendering. Need to be\n          called after finishing rendering or\n          cancel the rendering.\n Parameters:\n          page        -   Handle to the page, as returned by FPDF_LoadPage().\n Return value:\n          None."]
    #[allow(non_snake_case)]
    fn FPDF_RenderPage_Close(&self, page: FPDF_PAGE);

    #[doc = " Experimental API.\n Import pages to a FPDF_DOCUMENT.\n\n   dest_doc     - The destination document for the pages.\n   src_doc      - The document to be imported.\n   page_indices - An array of page indices to be imported. The first page is\n                  zero. If |page_indices| is NULL, all pages from |src_doc|\n                  are imported.\n   length       - The length of the |page_indices| array.\n   index        - The page index at which to insert the first imported page\n                  into |dest_doc|. The first page is zero.\n\n Returns TRUE on success. Returns FALSE if any pages in |page_indices| is\n invalid."]
    #[allow(non_snake_case)]
    fn FPDF_ImportPagesByIndex(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        page_indices: *const c_int,
        length: c_ulong,
        index: c_int,
    ) -> FPDF_BOOL;

    // TODO: AJRC - 24-Aug-24 - need doc comment for helper function
    #[inline]
    #[allow(non_snake_case)]
    fn FPDF_ImportPagesByIndex_vec(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        page_indices: Vec<c_int>,
        index: c_int,
    ) -> FPDF_BOOL {
        self.FPDF_ImportPagesByIndex(
            dest_doc,
            src_doc,
            page_indices.as_ptr(),
            page_indices.len() as c_ulong,
            index,
        )
    }

    #[doc = " Import pages to a FPDF_DOCUMENT.\n\n   dest_doc  - The destination document for the pages.\n   src_doc   - The document to be imported.\n   pagerange - A page range string, Such as \"1,3,5-7\". The first page is one.\n               If |pagerange| is NULL, all pages from |src_doc| are imported.\n   index     - The page index at which to insert the first imported page into\n               |dest_doc|. The first page is zero.\n\n Returns TRUE on success. Returns FALSE if any pages in |pagerange| is\n invalid or if |pagerange| cannot be read."]
    #[allow(non_snake_case)]
    fn FPDF_ImportPages(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        pagerange: &str,
        index: c_int,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Create a new document from |src_doc|.  The pages of |src_doc| will be\n combined to provide |num_pages_on_x_axis x num_pages_on_y_axis| pages per\n |output_doc| page.\n\n   src_doc             - The document to be imported.\n   output_width        - The output page width in PDF \"user space\" units.\n   output_height       - The output page height in PDF \"user space\" units.\n   num_pages_on_x_axis - The number of pages on X Axis.\n   num_pages_on_y_axis - The number of pages on Y Axis.\n\n Return value:\n   A handle to the created document, or NULL on failure.\n\n Comments:\n   number of pages per page = num_pages_on_x_axis * num_pages_on_y_axis\n"]
    #[allow(non_snake_case)]
    fn FPDF_ImportNPagesToOne(
        &self,
        src_doc: FPDF_DOCUMENT,
        output_width: c_float,
        output_height: c_float,
        num_pages_on_x_axis: size_t,
        num_pages_on_y_axis: size_t,
    ) -> FPDF_DOCUMENT;

    #[doc = " Experimental API.\n Create a template to generate form xobjects from |src_doc|'s page at\n |src_page_index|, for use in |dest_doc|.\n\n Returns a handle on success, or NULL on failure. Caller owns the newly\n created object."]
    #[allow(non_snake_case)]
    fn FPDF_NewXObjectFromPage(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
        src_page_index: c_int,
    ) -> FPDF_XOBJECT;

    #[doc = " Experimental API.\n Close an FPDF_XOBJECT handle created by FPDF_NewXObjectFromPage().\n FPDF_PAGEOBJECTs created from the FPDF_XOBJECT handle are not affected."]
    #[allow(non_snake_case)]
    fn FPDF_CloseXObject(&self, xobject: FPDF_XOBJECT);

    #[doc = " Experimental API.\n Create a new form object from an FPDF_XOBJECT object.\n\n Returns a new form object on success, or NULL on failure. Caller owns the\n newly created object."]
    #[allow(non_snake_case)]
    fn FPDF_NewFormObjectFromXObject(&self, xobject: FPDF_XOBJECT) -> FPDF_PAGEOBJECT;

    #[doc = " Copy the viewer preferences from |src_doc| into |dest_doc|.\n\n   dest_doc - Document to write the viewer preferences into.\n   src_doc  - Document to read the viewer preferences from.\n\n Returns TRUE on success."]
    #[allow(non_snake_case)]
    fn FPDF_CopyViewerPreferences(
        &self,
        dest_doc: FPDF_DOCUMENT,
        src_doc: FPDF_DOCUMENT,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API\n Function: FPDF_GetPageWidthF\n          Get page width.\n Parameters:\n          page        -   Handle to the page. Returned by FPDF_LoadPage().\n Return value:\n          Page width (excluding non-displayable area) measured in points.\n          One point is 1/72 inch (around 0.3528 mm)."]
    #[allow(non_snake_case)]
    fn FPDF_GetPageWidthF(&self, page: FPDF_PAGE) -> c_float;

    #[doc = " Function: FPDF_GetPageWidth\n          Get page width.\n Parameters:\n          page        -   Handle to the page. Returned by FPDF_LoadPage.\n Return value:\n          Page width (excluding non-displayable area) measured in points.\n          One point is 1/72 inch (around 0.3528 mm).\n Note:\n          Prefer FPDF_GetPageWidthF() above. This will be deprecated in the\n          future."]
    #[deprecated(
        since = "0.8.25",
        note = "Deprecated in favour of FPDF_GetPageWidthF()"
    )]
    #[allow(non_snake_case)]
    fn FPDF_GetPageWidth(&self, page: FPDF_PAGE) -> f64;

    #[doc = " Experimental API\n Function: FPDF_GetPageHeightF\n          Get page height.\n Parameters:\n          page        -   Handle to the page. Returned by FPDF_LoadPage().\n Return value:\n          Page height (excluding non-displayable area) measured in points.\n          One point is 1/72 inch (around 0.3528 mm)"]
    #[allow(non_snake_case)]
    fn FPDF_GetPageHeightF(&self, page: FPDF_PAGE) -> c_float;

    #[doc = " Function: FPDF_GetPageHeight\n          Get page height.\n Parameters:\n          page        -   Handle to the page. Returned by FPDF_LoadPage.\n Return value:\n          Page height (excluding non-displayable area) measured in points.\n          One point is 1/72 inch (around 0.3528 mm)\n Note:\n          Prefer FPDF_GetPageHeightF() above. This will be deprecated in the\n          future."]
    #[deprecated(
        since = "0.8.25",
        note = "Deprecated in favour of FPDF_GetPageHeightF()"
    )]
    #[allow(non_snake_case)]
    fn FPDF_GetPageHeight(&self, page: FPDF_PAGE) -> f64;

    #[allow(non_snake_case)]
    fn FPDFText_GetCharIndexFromTextIndex(
        &self,
        text_page: FPDF_TEXTPAGE,
        nTextIndex: c_int,
    ) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFText_GetTextIndexFromCharIndex(
        &self,
        text_page: FPDF_TEXTPAGE,
        nCharIndex: c_int,
    ) -> c_int;

    #[allow(non_snake_case)]
    fn FPDF_GetSignatureCount(&self, document: FPDF_DOCUMENT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDF_GetSignatureObject(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_SIGNATURE;

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetContents(
        &self,
        signature: FPDF_SIGNATURE,
        buffer: *mut c_void,
        length: c_ulong,
    ) -> c_ulong;

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetByteRange(
        &self,
        signature: FPDF_SIGNATURE,
        buffer: *mut c_int,
        length: c_ulong,
    ) -> c_ulong;

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetSubFilter(
        &self,
        signature: FPDF_SIGNATURE,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong;

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetReason(
        &self,
        signature: FPDF_SIGNATURE,
        buffer: *mut c_void,
        length: c_ulong,
    ) -> c_ulong;

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetTime(
        &self,
        signature: FPDF_SIGNATURE,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong;

    #[allow(non_snake_case)]
    fn FPDFSignatureObj_GetDocMDPPermission(&self, signature: FPDF_SIGNATURE) -> c_uint;

    /// Gets the structure tree for a page.
    ///
    ///   `page`        -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    /// Return value: a handle to the structure tree, or `NULL` on error. The caller owns the
    /// returned handle and must use [PdfiumLibraryBindings::FPDF_StructTree_Close] to release it.
    ///
    /// The handle should be released before `page` is released.
    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetForPage(&self, page: FPDF_PAGE) -> FPDF_STRUCTTREE;

    /// Releases a resource allocated by [PdfiumLibraryBindings::FPDF_StructTree_GetForPage].
    ///
    ///   `struct_tree` -   Handle to the structure tree, as returned by
    ///                     [PdfiumLibraryBindings::FPDF_StructTree_GetForPage].
    #[allow(non_snake_case)]
    fn FPDF_StructTree_Close(&self, struct_tree: FPDF_STRUCTTREE);

    /// Counts the number of children for the structure tree.
    ///
    ///   `struct_tree` -   Handle to the structure tree, as returned by
    ///                     [PdfiumLibraryBindings::FPDF_StructTree_GetForPage].
    ///
    /// Return value: the number of children, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructTree_CountChildren(&self, struct_tree: FPDF_STRUCTTREE) -> c_int;

    /// Gets a child in the structure tree.
    ///
    ///   `struct_tree` -   Handle to the structure tree, as returned by
    ///                     [PdfiumLibraryBindings::FPDF_StructTree_GetForPage].
    ///
    ///   `index`       -   The index for the child, 0-based.
    ///
    /// Return value: the child at the n-th index or `NULL` on error. The caller does not
    /// own the handle. The handle remains valid as long as `struct_tree` remains valid.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructTree_CountChildren] return value.
    #[allow(non_snake_case)]
    fn FPDF_StructTree_GetChildAtIndex(
        &self,
        struct_tree: FPDF_STRUCTTREE,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT;

    /// Gets the alt text for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `buffer`         -   A buffer for output the alt text. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the alt text, including the terminating `NUL` character.
    /// The number of bytes is returned regardless of the `buffer` and `buflen` parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAltText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the actual text for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `buffer`         -   A buffer for output the actual text. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the actual text, including the terminating `NUL` character.
    /// The number of bytes is returned regardless of the `buffer` and `buflen` parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetActualText(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the ID for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `buffer`         -   A buffer for output the ID string. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the ID string, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetID(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the case-insensitive IETF BCP 47 language code for an element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `buffer`         -   A buffer for output the lang string. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the ID string, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetLang(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets a struct element attribute of type `name` or `string`.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `attr_name`      -   The name of the attribute to retrieve.
    ///
    ///   `buffer`         -   A buffer for output. May be `NULL`.
    ///
    ///   `buflen`         -   The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the attribute value, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetStringAttribute(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        attr_name: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the marked content ID for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the marked content ID of the element. If no ID exists, returns 1.
    ///
    /// [PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentIdAtIndex] may be able to
    /// extract more marked content IDs out of `struct_element`. This API may be deprecated
    /// in the future.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentID(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int;

    /// Gets the type (/S) for a given element.
    ///
    ///   `struct_element` - Handle to the struct element.
    ///
    ///   `buffer`         - A buffer for output. May be `NULL`.
    ///
    ///   `buflen`         - The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the type, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the object type (/Type) for a given element.
    ///
    ///   `struct_element` - Handle to the struct element.
    ///
    ///   `buffer`         - A buffer for output. May be `NULL`.
    ///
    ///   `buflen`         - The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the object type, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and `buflen`
    /// parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetObjType(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the title (/T) for a given element.
    ///
    ///   `struct_element` - Handle to the struct element.
    ///
    ///   `buffer`         - A buffer for output. May be `NULL`.
    ///
    ///   `buflen`         - The length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the title, including the terminating `NUL` character.
    /// The number of bytes is returned regardless of the `buffer` and `buflen` parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding.
    /// The string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetTitle(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Counts the number of children for the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the number of children, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_CountChildren(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int;

    /// Gets a child in the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `index`          -   The index for the child, 0-based.
    ///
    /// Returns the child at the n-th index, or `NULL` on error.
    ///
    /// If the child exists but is not an element, then this function will return `NULL`.
    /// This will also return `NULL` for out-of-bounds indices.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_CountChildren]
    /// return value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT;

    #[cfg(any(
        feature = "pdfium_6084",
        feature = "pdfium_6110",
        feature = "pdfium_6124",
        feature = "pdfium_6164",
        feature = "pdfium_6259",
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406",
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Gets the child's content id.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `index`          -   The index for the child, 0-based.
    ///
    /// Returns the marked content ID of the child. If no ID exists, returns -1.
    ///
    /// If the child exists but is not a stream or object, then this function will return -1.
    /// This will also return -1 for out of bounds indices. Compared to
    /// [PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentIdAtIndex],
    /// it is scoped to the current page.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_CountChildren]
    /// return value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetChildMarkedContentID(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> c_int;

    /// Gets the parent of the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the parent structure element, or `NULL` on error.
    ///
    /// If structure element is StructTreeRoot, then this function will return `NULL`.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetParent(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
    ) -> FPDF_STRUCTELEMENT;

    /// Counts the number of attributes for the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the number of attributes, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeCount(&self, struct_element: FPDF_STRUCTELEMENT) -> c_int;

    /// Gets an attribute object in the structure element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `index`          -   The index for the attribute object, 0-based.
    ///
    /// Returns the attribute object at the n-th index, or `NULL` on error.
    ///
    /// If the attribute object exists but is not a dict, then this function will return `NULL`.
    /// This will also return `NULL` for out-of-bounds indices. The caller does not own the handle.
    /// The handle remains valid as long as `struct_element` remains valid.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_GetAttributeCount]
    /// return value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetAttributeAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR;

    /// Counts the number of attributes in a structure element attribute map.
    ///
    ///   `struct_attribute` - Handle to the struct element attribute.
    ///
    /// Returns the number of attributes, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetCount(&self, struct_attribute: FPDF_STRUCTELEMENT_ATTR) -> c_int;

    /// Gets the name of an attribute in a structure element attribute map.
    ///
    ///   `struct_attribute`   - Handle to the struct element attribute.
    ///
    ///   `index`              - The index of attribute in the map.
    ///
    ///   `buffer`             - A buffer for output. May be `NULL`. This is only
    ///                          modified if `buflen` is longer than the length
    ///                          of the key. Optional, pass `NULL` to just
    ///                          retrieve the size of the buffer needed.
    ///
    ///   `buflen`             - The length of the buffer.
    ///
    ///   `out_buflen`         - A pointer to variable that will receive the
    ///                          minimum buffer size to contain the key. Not
    ///                          filled if `FALSE` is returned.
    ///
    /// Returns `TRUE` if the operation was successful, `FALSE` otherwise.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetName(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Gets a handle to a value for an attribute in a structure element attribute map.
    ///
    ///   `struct_attribute`   - Handle to the struct element attribute.
    ///
    ///   `name`               - The attribute name.
    ///
    /// Returns a handle to the value associated with the input, if any. Returns `NULL`
    /// on failure. The caller does not own the handle.
    ///
    /// The handle remains valid as long as `struct_attribute` remains valid.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetValue(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE;

    #[cfg(any(
        feature = "pdfium_5961",
        feature = "pdfium_6015",
        feature = "pdfium_6043",
        feature = "pdfium_6084",
        feature = "pdfium_6110",
        feature = "pdfium_6124",
        feature = "pdfium_6164",
        feature = "pdfium_6259",
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406"
    ))]
    /// Gets the type of an attribute in a structure element attribute map.
    ///
    ///   `struct_attribute`   - Handle to the struct element attribute.
    ///
    ///   `name`               - The attribute name.
    ///
    /// Returns the type of the value, or `FPDF_OBJECT_UNKNOWN` in case of failure.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetType(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
    ) -> FPDF_OBJECT_TYPE;

    #[cfg(any(
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Gets the type of an attribute in a structure element attribute map.
    ///
    ///   `value` - Handle to the value.
    ///
    /// Returns the type of the value, or `FPDF_OBJECT_UNKNOWN` in case of failure. Note that
    /// this will never return `FPDF_OBJECT_REFERENCE`, as references are always dereferenced.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetType(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
    ) -> FPDF_OBJECT_TYPE;

    #[cfg(any(
        feature = "pdfium_5961",
        feature = "pdfium_6015",
        feature = "pdfium_6043",
        feature = "pdfium_6084",
        feature = "pdfium_6110",
        feature = "pdfium_6124",
        feature = "pdfium_6164",
        feature = "pdfium_6259",
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406"
    ))]
    /// Gets the value of a boolean attribute in an attribute map by name as `FPDF_BOOL`.
    /// [PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType] should have returned
    /// `FPDF_OBJECT_BOOLEAN` for this property.
    ///
    ///   `struct_attribute`   - Handle to the struct element attribute.
    ///
    ///   `name`               - The attribute name.
    ///
    ///   `out_value`          - A pointer to variable that will receive the
    ///                          value. Not filled if `FALSE` is returned.
    ///
    /// Returns `TRUE` if the name maps to a boolean value, `FALSE` otherwise.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBooleanValue(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
        out_value: *mut FPDF_BOOL,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Gets the value of a boolean attribute in an attribute map as `FPDF_BOOL`.
    /// [PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType] should have returned
    /// `FPDF_OBJECT_BOOLEAN` for this property.
    ///
    ///   `value`     - Handle to the value.
    ///
    ///   `out_value` - A pointer to variable that will receive the value. Not
    ///                 filled if false is returned.
    ///
    /// Returns `TRUE` if the attribute maps to a boolean value, `FALSE` otherwise.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBooleanValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut FPDF_BOOL,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_5961",
        feature = "pdfium_6015",
        feature = "pdfium_6043",
        feature = "pdfium_6084",
        feature = "pdfium_6110",
        feature = "pdfium_6124",
        feature = "pdfium_6164",
        feature = "pdfium_6259",
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406"
    ))]
    /// Gets the value of a number attribute in an attribute map by name as float.
    /// [PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType] should have returned
    /// `FPDF_OBJECT_NUMBER` for this property.
    ///
    ///   `struct_attribute`   - Handle to the struct element attribute.
    ///
    ///   `name`               - The attribute name.
    ///
    ///   `out_value`          - A pointer to variable that will receive the
    ///                          value. Not filled if `FALSE` is returned.
    ///
    /// Returns `TRUE` if the name maps to a number value, `FALSE` otherwise.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetNumberValue(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
        out_value: *mut f32,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Gets the value of a number attribute in an attribute map as float.
    /// [PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType] should have returned
    /// `FPDF_OBJECT_NUMBER` for this property.
    ///
    ///   `value`     - Handle to the value.
    ///
    ///   `out_value` - A pointer to variable that will receive the value. Not
    ///                 filled if false is returned.
    ///
    /// Returns `TRUE` if the attribute maps to a number value, `FALSE` otherwise.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetNumberValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        out_value: *mut f32,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_5961",
        feature = "pdfium_6015",
        feature = "pdfium_6043",
        feature = "pdfium_6084",
        feature = "pdfium_6110",
        feature = "pdfium_6124",
        feature = "pdfium_6164",
        feature = "pdfium_6259",
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406"
    ))]
    /// Gets the value of a string attribute in an attribute map by name as string.
    /// [PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType] should have returned
    /// `FPDF_OBJECT_STRING` or `FPDF_OBJECT_NAME` for this property.
    ///
    ///   `struct_attribute`   - Handle to the struct element attribute.
    ///
    ///   `name`               - The attribute name.
    ///
    ///   `buffer`             - A buffer for holding the returned key in
    ///                          UTF-16LE. This is only modified if `buflen` is
    ///                          longer than the length of the key. Optional,
    ///                          pass `NULL` to just retrieve the size of the
    ///                          buffer needed.
    ///
    ///   `buflen`             - The length of the buffer.
    ///
    ///   `out_buflen`         - A pointer to variable that will receive the
    ///                          minimum buffer size to contain the key. Not
    ///                          filled if `FALSE` is returned.
    ///
    /// Returns `TRUE` if the name maps to a string value, `FALSE` otherwise.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetStringValue(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Gets the value of a string attribute in an attribute map as string.
    /// [PdfiumLibraryBindings::FPDF_StructElement_Attr_GetType] should have returned
    /// `FPDF_OBJECT_STRING` or `FPDF_OBJECT_NAME` for this property.
    ///
    ///   `value`      - Handle to the value.
    ///
    ///   `buffer`     - A buffer for holding the returned key in UTF-16LE.
    ///                  This is only modified if `buflen` is longer than the
    ///                  length of the key. Optional, pass `NULL` to just
    ///                  retrieve the size of the buffer needed.
    ///
    ///   `buflen`     - The length of the buffer.
    ///
    ///   `out_buflen` - A pointer to variable that will receive the minimum
    ///                  buffer size to contain the key. Not filled if `FALSE` is
    ///                  returned.
    ///
    /// Returns `TRUE` if the attribute maps to a string value, `FALSE` otherwise.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetStringValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_5961",
        feature = "pdfium_6015",
        feature = "pdfium_6043",
        feature = "pdfium_6084",
        feature = "pdfium_6110",
        feature = "pdfium_6124",
        feature = "pdfium_6164",
        feature = "pdfium_6259",
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406"
    ))]
    /// Gets the value of a blob attribute in an attribute map by name as string.
    ///
    ///   `struct_attribute`   - Handle to the struct element attribute.
    ///
    ///   `name`               - The attribute name.
    ///
    ///   `buffer`             - A buffer for holding the returned value. This
    ///                          is only modified if |buflen| is at least as
    ///                          long as the length of the value. Optional, pass
    ///                          `NULL` to just retrieve the size of the buffer
    ///                          needed.
    ///
    ///   `buflen`             - The length of the buffer.
    ///
    ///   `out_buflen`         - A pointer to variable that will receive the
    ///                          minimum buffer size to contain the key. Not
    ///                          filled if `FALSE` is returned.
    ///
    /// Returns `TRUE` if the name maps to a string value, `FALSE` otherwise.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBlobValue(
        &self,
        struct_attribute: FPDF_STRUCTELEMENT_ATTR,
        name: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Gets the value of a blob attribute in an attribute map as string.
    ///
    ///   `value`      - Handle to the value.
    ///
    ///   `buffer`     - A buffer for holding the returned value. This is only
    ///                  modified if `buflen` is at least as long as the length
    ///                  of the value. Optional, pass `NULL` to just retrieve the
    ///                  size of the buffer needed.
    ///
    ///   `buflen`     - The length of the buffer.
    ///
    ///   `out_buflen` - A pointer to variable that will receive the minimum buffer size
    ///                  to contain the key. Not filled if `FALSE` is returned.
    ///
    /// Returns `TRUE` if the attribute maps to a string value, `FALSE` otherwise.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetBlobValue(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Counts the number of children values in an attribute.
    ///
    ///   `value` - Handle to the value.
    ///
    /// Returns the number of children, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_CountChildren(&self, value: FPDF_STRUCTELEMENT_ATTR_VALUE) -> c_int;

    #[cfg(any(
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Gets a child from an attribute.
    ///
    ///   `value` - Handle to the value.
    ///
    ///   `index` - The index for the child, 0-based.
    ///
    /// Returns the child at the n-th index, or `NULL` on error.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_Attr_CountChildren]
    /// return value.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_Attr_GetChildAtIndex(
        &self,
        value: FPDF_STRUCTELEMENT_ATTR_VALUE,
        index: c_int,
    ) -> FPDF_STRUCTELEMENT_ATTR_VALUE;

    /// Gets the count of marked content ids for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    /// Returns the count of marked content ids or -1 if none exists.
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdCount(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
    ) -> c_int;

    /// Gets the marked content id at a given index for a given element.
    ///
    ///   `struct_element` -   Handle to the struct element.
    ///
    ///   `index`          -   The index of the marked content id, 0-based.
    ///
    /// Returns the marked content ID of the element. If no ID exists, returns -1.
    ///
    /// The `index` must be less than the [PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentIdCount]
    /// return value.
    ///
    /// This function will likely supersede [PdfiumLibraryBindings::FPDF_StructElement_GetMarkedContentID].
    #[allow(non_snake_case)]
    fn FPDF_StructElement_GetMarkedContentIdAtIndex(
        &self,
        struct_element: FPDF_STRUCTELEMENT,
        index: c_int,
    ) -> c_int;

    #[doc = " Create a new PDF page.\n\n   document   - handle to document.\n   page_index - suggested 0-based index of the page to create. If it is larger\n                than document's current last index(L), the created page index\n                is the next available index -- L+1.\n   width      - the page width in points.\n   height     - the page height in points.\n\n Returns the handle to the new page or NULL on failure.\n\n The page should be closed with FPDF_ClosePage() when finished as\n with any other page in the document."]
    #[allow(non_snake_case)]
    fn FPDFPage_New(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        width: c_double,
        height: c_double,
    ) -> FPDF_PAGE;

    #[doc = " Delete the page at |page_index|.\n\n   document   - handle to document.\n   page_index - the index of the page to delete."]
    #[allow(non_snake_case)]
    fn FPDFPage_Delete(&self, document: FPDF_DOCUMENT, page_index: c_int);

    #[cfg(any(
        feature = "pdfium_6043",
        feature = "pdfium_6084",
        feature = "pdfium_6110",
        feature = "pdfium_6124",
        feature = "pdfium_6164",
        feature = "pdfium_6259",
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406",
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[doc = " Experimental API.\n Move the given pages to a new index position.\n\n  page_indices     - the ordered list of pages to move. No duplicates allowed.\n  page_indices_len - the number of elements in |page_indices|\n  dest_page_index  - the new index position to which the pages in\n                     |page_indices| are moved.\n\n Returns TRUE on success. If it returns FALSE, the document may be left in an\n indeterminate state.\n\n Example: The PDF document starts out with pages [A, B, C, D], with indices\n [0, 1, 2, 3].\n\n >  Move(doc, [3, 2], 2, 1); // returns true\n >  // The document has pages [A, D, C, B].\n >\n >  Move(doc, [0, 4, 3], 3, 1); // returns false\n >  // Returned false because index 4 is out of range.\n >\n >  Move(doc, [0, 3, 1], 3, 2); // returns false\n >  // Returned false because index 2 is out of range for 3 page indices.\n >\n >  Move(doc, [2, 2], 2, 0); // returns false\n >  // Returned false because [2, 2] contains duplicates.\n"]
    #[allow(non_snake_case)]
    fn FPDF_MovePages(
        &self,
        document: FPDF_DOCUMENT,
        page_indices: *const c_int,
        page_indices_len: c_ulong,
        dest_page_index: c_int,
    ) -> FPDF_BOOL;

    #[doc = " Get the rotation of |page|.\n\n   page - handle to a page\n\n Returns one of the following indicating the page rotation:\n   0 - No rotation.\n   1 - Rotated 90 degrees clockwise.\n   2 - Rotated 180 degrees clockwise.\n   3 - Rotated 270 degrees clockwise."]
    #[allow(non_snake_case)]
    fn FPDFPage_GetRotation(&self, page: FPDF_PAGE) -> c_int;

    #[doc = " Set rotation for |page|.\n\n   page   - handle to a page.\n   rotate - the rotation value, one of:\n              0 - No rotation.\n              1 - Rotated 90 degrees clockwise.\n              2 - Rotated 180 degrees clockwise.\n              3 - Rotated 270 degrees clockwise."]
    #[allow(non_snake_case)]
    fn FPDFPage_SetRotation(&self, page: FPDF_PAGE, rotate: c_int);

    #[doc = " Experimental API.\n Function: FPDF_GetPageBoundingBox\n          Get the bounding box of the page. This is the intersection between\n          its media box and its crop box.\n Parameters:\n          page        -   Handle to the page. Returned by FPDF_LoadPage.\n          rect        -   Pointer to a rect to receive the page bounding box.\n                          On an error, |rect| won't be filled.\n Return value:\n          True for success."]
    #[allow(non_snake_case)]
    fn FPDF_GetPageBoundingBox(&self, page: FPDF_PAGE, rect: *mut FS_RECTF) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FPDF_GetPageSizeByIndexF\n          Get the size of the page at the given index.\n Parameters:\n          document    -   Handle to document. Returned by FPDF_LoadDocument().\n          page_index  -   Page index, zero for the first page.\n          size        -   Pointer to a FS_SIZEF to receive the page size.\n                          (in points).\n Return value:\n          Non-zero for success. 0 for error (document or page not found)."]
    #[allow(non_snake_case)]
    fn FPDF_GetPageSizeByIndexF(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        size: *mut FS_SIZEF,
    ) -> FPDF_BOOL;

    #[doc = " Function: FPDF_GetPageSizeByIndex\n          Get the size of the page at the given index.\n Parameters:\n          document    -   Handle to document. Returned by FPDF_LoadDocument.\n          page_index  -   Page index, zero for the first page.\n          width       -   Pointer to a double to receive the page width\n                          (in points).\n          height      -   Pointer to a double to receive the page height\n                          (in points).\n Return value:\n          Non-zero for success. 0 for error (document or page not found).\n Note:\n          Prefer FPDF_GetPageSizeByIndexF() above. This will be deprecated in\n          the future."]
    #[allow(non_snake_case)]
    fn FPDF_GetPageSizeByIndex(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        width: *mut f64,
        height: *mut f64,
    ) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPage_GetMediaBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPage_GetCropBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPage_GetBleedBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPage_GetTrimBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPage_GetArtBox(
        &self,
        page: FPDF_PAGE,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPage_SetMediaBox(
        &self,
        page: FPDF_PAGE,
        left: c_float,
        bottom: c_float,
        right: c_float,
        top: c_float,
    );

    #[allow(non_snake_case)]
    fn FPDFPage_SetCropBox(
        &self,
        page: FPDF_PAGE,
        left: c_float,
        bottom: c_float,
        right: c_float,
        top: c_float,
    );

    #[allow(non_snake_case)]
    fn FPDFPage_SetBleedBox(
        &self,
        page: FPDF_PAGE,
        left: c_float,
        bottom: c_float,
        right: c_float,
        top: c_float,
    );

    #[allow(non_snake_case)]
    fn FPDFPage_SetTrimBox(
        &self,
        page: FPDF_PAGE,
        left: c_float,
        bottom: c_float,
        right: c_float,
        top: c_float,
    );

    #[allow(non_snake_case)]
    fn FPDFPage_SetArtBox(
        &self,
        page: FPDF_PAGE,
        left: c_float,
        bottom: c_float,
        right: c_float,
        top: c_float,
    );

    #[allow(non_snake_case)]
    fn FPDFPage_TransFormWithClip(
        &self,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipRect: *const FS_RECTF,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFPageObj_TransformClipPath(
        &self,
        page_object: FPDF_PAGEOBJECT,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    );

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetClipPath(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_CLIPPATH;

    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPaths(&self, clip_path: FPDF_CLIPPATH) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFClipPath_CountPathSegments(&self, clip_path: FPDF_CLIPPATH, path_index: c_int) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFClipPath_GetPathSegment(
        &self,
        clip_path: FPDF_CLIPPATH,
        path_index: c_int,
        segment_index: c_int,
    ) -> FPDF_PATHSEGMENT;

    #[allow(non_snake_case)]
    fn FPDF_CreateClipPath(&self, left: f32, bottom: f32, right: f32, top: f32) -> FPDF_CLIPPATH;

    #[allow(non_snake_case)]
    fn FPDF_DestroyClipPath(&self, clipPath: FPDF_CLIPPATH);

    #[allow(non_snake_case)]
    fn FPDFPage_InsertClipPath(&self, page: FPDF_PAGE, clipPath: FPDF_CLIPPATH);

    #[allow(non_snake_case)]
    fn FPDFPage_HasTransparency(&self, page: FPDF_PAGE) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPage_GenerateContent(&self, page: FPDF_PAGE) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFPage_TransformAnnots(
        &self,
        page: FPDF_PAGE,
        a: f64,
        b: f64,
        c: f64,
        d: f64,
        e: f64,
        f: f64,
    );

    /// Creates a device independent bitmap (FXDIB).
    ///
    ///   `width`       -   The number of pixels in width for the bitmap.
    ///                     Must be greater than 0.
    ///
    ///   `height`      -   The number of pixels in height for the bitmap.
    ///                     Must be greater than 0.
    ///
    ///   `alpha`       -   A flag indicating whether the alpha channel is used.
    ///                     Non-zero for using alpha, zero for not using.
    ///
    /// Returns the created bitmap handle, or `NULL` if a parameter error or out of
    /// memory.
    ///
    /// The bitmap always uses 4 bytes per pixel. The first byte is always double word aligned.
    /// The byte order is BGRx (the last byte unused if no alpha channel) or BGRA.
    /// The pixels in a horizontal line are stored side by side, with the left most pixel
    /// stored first (with lower memory address). Each line uses `width * 4` bytes.
    /// Lines are stored one after another, with the top most line stored first.
    /// There is no gap between adjacent lines.
    ///
    /// This function allocates enough memory for holding all pixels in the bitmap,
    /// but it doesn't initialize the buffer. Applications can use [PdfiumLibraryBindings::FPDFBitmap_FillRect]
    /// to fill the bitmap using any color. If the OS allows it, this function can allocate
    /// up to 4 GB of memory.
    #[allow(non_snake_case)]
    fn FPDFBitmap_Create(&self, width: c_int, height: c_int, alpha: c_int) -> FPDF_BITMAP;

    /// Creates a device independent bitmap (FXDIB).
    ///
    ///   `width`       -   The number of pixels in width for the bitmap.
    ///                     Must be greater than 0.
    ///
    ///   `height`      -   The number of pixels in height for the bitmap.
    ///                     Must be greater than 0.
    ///
    ///   `format`      -   A number indicating for bitmap format, as defined above.
    ///
    ///   `first_scan`  -   A pointer to the first byte of the first line if
    ///                     using an external buffer. If this parameter is `NULL`,
    ///                     then a new buffer will be created.
    ///
    ///   `stride`      -   Number of bytes for each scan line. The value must
    ///                     be 0 or greater. When the value is 0,
    ///                     `FPDFBitmap_CreateEx()` will automatically calculate
    ///                     the appropriate value using `width` and `format`.
    ///                     When using an external buffer, it is recommended for the caller
    ///                     to pass in the value. When not using an external buffer, it is
    ///                     recommended for the caller to pass in 0.
    ///
    /// Returns the bitmap handle, or `NULL` if parameter error or out of memory.
    ///
    /// Similar to [PdfiumLibraryBindings::FPDFBitmap_Create] function, but allows for more
    /// formats and an external buffer is supported. The bitmap created by this function
    /// can be used in any place that a `FPDF_BITMAP` handle is required.
    ///
    /// If an external buffer is used, then the caller should destroy the buffer.
    /// [PdfiumLibraryBindings::FPDFBitmap_Destroy] will not destroy the buffer.
    ///
    /// It is recommended to use [PdfiumLibraryBindings::FPDFBitmap_GetStride to get the stride
    /// value.
    #[allow(non_snake_case)]
    fn FPDFBitmap_CreateEx(
        &self,
        width: c_int,
        height: c_int,
        format: c_int,
        first_scan: *mut c_void,
        stride: c_int,
    ) -> FPDF_BITMAP;

    /// Gets the format of the bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the format of the bitmap.
    ///
    /// Only formats supported by [PdfiumLibraryBindings::FPDFBitmap_CreateEx] are supported by this
    /// function; see the list of such formats above.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetFormat(&self, bitmap: FPDF_BITMAP) -> c_int;

    #[cfg(any(
        feature = "pdfium_6611",
        feature = "pdfium_6569",
        feature = "pdfium_6555",
        feature = "pdfium_6490",
        feature = "pdfium_6406",
        feature = "pdfium_6337",
        feature = "pdfium_6295",
        feature = "pdfium_6259",
        feature = "pdfium_6164",
        feature = "pdfium_6124",
        feature = "pdfium_6110",
        feature = "pdfium_6084",
        feature = "pdfium_6043",
        feature = "pdfium_6015",
        feature = "pdfium_5961"
    ))]
    /// Fills a rectangle in a bitmap.
    ///
    ///   `bitmap`      -   The handle to the bitmap. Returned by
    ///                     [PdfiumLibraryBindings::FPDFBitmap_Create].
    ///
    ///   `left`        -   The left position. Starting from 0 at the left-most pixel.
    ///
    ///   `top`         -   The top position. Starting from 0 at the top-most line.
    ///
    ///   `width`       -   Width in pixels to be filled.
    ///
    ///   `height`      -   Height in pixels to be filled.
    ///
    ///   `color`       -   A 32-bit value specifying the color, in 8888 ARGB format.
    ///
    /// This function sets the color and (optionally) alpha value in the specified region
    /// of the bitmap.
    ///
    /// Note: If the alpha channel is used, this function does _not_ composite the background
    /// with the source color, instead the background will be replaced by the source color
    /// and the alpha. If the alpha channel is not used, the alpha parameter is ignored.
    #[allow(non_snake_case)]
    fn FPDFBitmap_FillRect(
        &self,
        bitmap: FPDF_BITMAP,
        left: c_int,
        top: c_int,
        width: c_int,
        height: c_int,
        color: FPDF_DWORD,
    );

    #[cfg(any(feature = "pdfium_6666", feature = "pdfium_future"))]
    /// Fills a rectangle in a bitmap.
    ///
    ///   `bitmap`      -   The handle to the bitmap. Returned by
    ///                     [PdfiumLibraryBindings::FPDFBitmap_Create].
    ///
    ///   `left`        -   The left position. Starting from 0 at the left-most pixel.
    ///
    ///   `top`         -   The top position. Starting from 0 at the top-most line.
    ///
    ///   `width`       -   Width in pixels to be filled.
    ///
    ///   `height`      -   Height in pixels to be filled.
    ///
    ///   `color`       -   A 32-bit value specifying the color, in 8888 ARGB format.
    ///
    /// Returns whether the operation succeeded or not.
    ///
    /// This function sets the color and (optionally) alpha value in the specified region
    /// of the bitmap.
    ///
    /// Note: If the alpha channel is used, this function does _not_ composite the background
    /// with the source color, instead the background will be replaced by the source color
    /// and the alpha. If the alpha channel is not used, the alpha parameter is ignored.
    #[allow(non_snake_case)]
    fn FPDFBitmap_FillRect(
        &self,
        bitmap: FPDF_BITMAP,
        left: c_int,
        top: c_int,
        width: c_int,
        height: c_int,
        color: FPDF_DWORD,
    ) -> FPDF_BOOL;

    /// Note that the return type of this function is modified when compiling to WASM. Instead
    /// of returning `*mut c_void`, it returns `*const c_void`.
    ///
    /// When compiling to WASM, Pdfium's internal pixel data buffer for the given bitmap resides
    /// in a separate WASM memory module, so any buffer returned by this function is necessarily
    /// a copy; mutating that copy does not alter the buffer in Pdfium's WASM module and, since
    /// there is no way for `pdfium-render` to know when the caller has finished mutating the
    /// copied buffer, there is no reliable way for `pdfium-render` to transfer any changes made
    /// to the copy across to Pdfium's WASM module.
    ///
    /// To avoid having to maintain different code for different platform targets, it is
    /// recommended that all callers use the provided [PdfiumLibraryBindings::FPDFBitmap_SetBuffer]
    /// convenience function to apply modified pixel data to a bitmap instead of mutating the
    /// buffer returned by this function.
    ///
    /// Gets the data buffer of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the pointer to the first byte of the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    ///
    /// Applications can use this function to get the bitmap buffer pointer,
    /// then manipulate any color and/or alpha values for any pixels in the bitmap.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_GetFormat] to find out the format of the data.
    #[allow(non_snake_case)]
    #[cfg(not(target_arch = "wasm32"))]
    fn FPDFBitmap_GetBuffer(&self, bitmap: FPDF_BITMAP) -> *mut c_void;

    /// Note that the return type of this function is modified when compiling to WASM. Instead
    /// of returning `*mut c_void`, it returns `*const c_void`.
    ///
    /// When compiling to WASM, Pdfium's internal pixel data buffer for the given bitmap resides
    /// in a separate WASM memory module, so any buffer returned by this function is necessarily
    /// a copy; mutating that copy does not alter the buffer in Pdfium's WASM module and, since
    /// there is no way for `pdfium-render` to know when the caller has finished mutating the
    /// copied buffer, there is no reliable way for `pdfium-render` to transfer any changes made
    /// to the copy across to Pdfium's WASM module.
    ///
    /// **Do not mutate the pixel data in the buffer returned by this function.** Instead, use the
    /// [PdfiumLibraryBindings::FPDFBitmap_SetBuffer] function to apply a new pixel data
    /// buffer to the bitmap.
    ///
    /// Gets the data buffer of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the pointer to the first byte of the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    ///
    /// Applications can use this function to get the bitmap buffer pointer,
    /// then manipulate any color and/or alpha values for any pixels in the bitmap.
    ///
    /// Use [PdfiumLibraryBindings::FPDFBitmap_GetFormat] to find out the format of the data.
    #[allow(non_snake_case)]
    #[cfg(target_arch = "wasm32")]
    fn FPDFBitmap_GetBuffer(&self, bitmap: FPDF_BITMAP) -> *const c_void;

    /// This function is not part of the Pdfium API. It is provided by `pdfium-render` as an
    /// alternative to directly mutating the data returned by
    /// [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    ///
    /// Replaces all pixel data for the given bitmap with the pixel data in the given buffer,
    /// returning `true` once the new pixel data has been applied. If the given buffer
    /// does not have the same length as the bitmap's current buffer then the current buffer
    /// will be unchanged and a value of `false` will be returned.
    #[allow(non_snake_case)]
    #[cfg(not(target_arch = "wasm32"))]
    fn FPDFBitmap_SetBuffer(&self, bitmap: FPDF_BITMAP, buffer: &[u8]) -> bool {
        let buffer_length =
            (self.FPDFBitmap_GetStride(bitmap) * self.FPDFBitmap_GetHeight(bitmap)) as usize;

        if buffer.len() != buffer_length {
            return false;
        }

        let buffer_start = self.FPDFBitmap_GetBuffer(bitmap);

        let destination =
            unsafe { std::slice::from_raw_parts_mut(buffer_start as *mut u8, buffer_length) };

        destination.copy_from_slice(buffer);

        true
    }

    /// This function is not part of the Pdfium API. It is provided by `pdfium-render` as an
    /// alternative to directly mutating the data returned by
    /// [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    ///
    /// Replaces all pixel data of the given bitmap with the pixel data in the given buffer,
    /// returning `true` once the new pixel data has been applied. If the given buffer
    /// does not have the same length as the bitmap's current buffer then the current buffer
    /// will be unchanged and a value of `false` will be returned.
    #[allow(non_snake_case)]
    #[cfg(target_arch = "wasm32")]
    fn FPDFBitmap_SetBuffer(&self, bitmap: FPDF_BITMAP, buffer: &[u8]) -> bool;

    /// This function is not part of the Pdfium API. It is provided by `pdfium-render` as a
    /// more performant WASM-specific variant of [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    /// Since it avoids a (potentially large) bitmap allocation and copy, it is both faster and
    /// more memory efficient than [PdfiumLibraryBindings::FPDFBitmap_GetBuffer].
    ///
    /// This function is only available when compiling to WASM.
    #[allow(non_snake_case)]
    #[cfg(target_arch = "wasm32")]
    fn FPDFBitmap_GetArray(&self, bitmap: FPDF_BITMAP) -> js_sys::Uint8Array;

    /// Gets the width of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the width of the bitmap in pixels.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetWidth(&self, bitmap: FPDF_BITMAP) -> c_int;

    /// Gets the height of a bitmap.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the height of the bitmap in pixels.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetHeight(&self, bitmap: FPDF_BITMAP) -> c_int;

    /// Gets the number of bytes for each line in the bitmap buffer.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// Returns the number of bytes for each line in the bitmap buffer.
    ///
    /// The stride may be more than `width * number of bytes per pixel`.
    #[allow(non_snake_case)]
    fn FPDFBitmap_GetStride(&self, bitmap: FPDF_BITMAP) -> c_int;

    /// Destroys a bitmap and releases all related buffers.
    ///
    ///   `bitmap`      -   Handle to the bitmap. Returned by [PdfiumLibraryBindings::FPDFBitmap_Create]
    ///                     or [PdfiumLibraryBindings::FPDFImageObj_GetBitmap].
    ///
    /// This function will not destroy any external buffers provided when
    /// the bitmap was created.
    #[allow(non_snake_case)]
    fn FPDFBitmap_Destroy(&self, bitmap: FPDF_BITMAP);

    #[cfg(not(target_arch = "wasm32"))]
    #[cfg(feature = "pdfium_use_win32")]
    // Function: FPDF_RenderPage
    //          Render contents of a page to a device (screen, bitmap, or printer).
    //          This function is only supported on Windows.
    // Parameters:
    //          dc          -   Handle to the device context.
    //          page        -   Handle to the page. Returned by FPDF_LoadPage.
    //          start_x     -   Left pixel position of the display area in
    //                          device coordinates.
    //          start_y     -   Top pixel position of the display area in device
    //                          coordinates.
    //          size_x      -   Horizontal size (in pixels) for displaying the page.
    //          size_y      -   Vertical size (in pixels) for displaying the page.
    //          rotate      -   Page orientation:
    //                            0 (normal)
    //                            1 (rotated 90 degrees clockwise)
    //                            2 (rotated 180 degrees)
    //                            3 (rotated 90 degrees counter-clockwise)
    //          flags       -   0 for normal display, or combination of flags
    //                          defined above.
    // Return value:
    //          None.
    #[allow(non_snake_case)]
    fn FPDF_RenderPage(
        &self,
        dc: windows::Win32::Graphics::Gdi::HDC,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    );

    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDF_RenderPageBitmap(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    );

    #[allow(non_snake_case)]
    fn FPDF_RenderPageBitmapWithMatrix(
        &self,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        matrix: *const FS_MATRIX,
        clipping: *const FS_RECTF,
        flags: c_int,
    );

    #[cfg(feature = "pdfium_use_skia")]
    #[doc = " Experimental API.\n Function: FPDF_RenderPageSkia\n          Render contents of a page to a Skia SkCanvas.\n Parameters:\n          canvas      -   SkCanvas to render to.\n          page        -   Handle to the page.\n          size_x      -   Horizontal size (in pixels) for displaying the page.\n          size_y      -   Vertical size (in pixels) for displaying the page.\n Return value:\n          None."]
    #[allow(non_snake_case)]
    fn FPDF_RenderPageSkia(
        &self,
        canvas: FPDF_SKIA_CANVAS,
        page: FPDF_PAGE,
        size_x: c_int,
        size_y: c_int,
    );

    /// Checks if an annotation subtype is currently supported for creation.
    /// Currently supported subtypes:
    ///
    ///    - circle
    ///
    ///    - file attachment
    ///
    ///    - freetext
    ///
    ///    - highlight
    ///
    ///    - ink
    ///
    ///    - link
    ///
    ///    - popup
    ///
    ///    - square
    ///
    ///    - squiggly
    ///
    ///    - stamp
    ///
    ///    - strikeout
    ///
    ///    - text
    ///
    ///    - underline
    ///
    ///   `subtype`   - the subtype to be checked.
    ///
    /// Returns `true` if this subtype supported.
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL;

    /// Creates an annotation in `page` of the subtype `subtype`. If the specified
    /// subtype is illegal or unsupported, then a new annotation will not be created.
    /// Must call [PdfiumLibraryBindings::FPDFPage_CloseAnnot] when the annotation returned by this
    /// function is no longer needed.
    ///
    ///   `page`      - handle to a page.
    ///
    ///   `subtype`   - the subtype of the new annotation.
    ///
    /// Returns a handle to the new annotation object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFPage_CreateAnnot(
        &self,
        page: FPDF_PAGE,
        subtype: FPDF_ANNOTATION_SUBTYPE,
    ) -> FPDF_ANNOTATION;

    /// Gets the number of annotations in `page`.
    ///
    ///   `page`   - handle to a page.
    ///
    /// Returns the number of annotations in `page`.
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotCount(&self, page: FPDF_PAGE) -> c_int;

    /// Gets annotation in `page` at `index`. Must call [PdfiumLibraryBindings::FPDFPage_CloseAnnot] when the
    /// annotation returned by this function is no longer needed.
    ///
    ///   `page`  - handle to a page.
    ///
    ///   `index` - the index of the annotation.
    ///
    /// Returns a handle to the annotation object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_ANNOTATION;

    /// Gets the index of `annot` in `page`. This is the opposite of
    /// [PdfiumLibraryBindings::FPDFPage_GetAnnot].
    ///
    ///   `page`  - handle to the page that the annotation is on.
    ///
    ///   `annot` - handle to an annotation.
    ///
    /// Returns the index of `annot`, or -1 on failure.
    #[allow(non_snake_case)]
    fn FPDFPage_GetAnnotIndex(&self, page: FPDF_PAGE, annot: FPDF_ANNOTATION) -> c_int;

    /// Closes an annotation. Must be called when the annotation returned by
    /// [PdfiumLibraryBindings::FPDFPage_CreateAnnot] or [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    /// is no longer needed. This function does not remove the annotation from the document.
    ///
    ///   `annot`  - handle to an annotation.
    #[allow(non_snake_case)]
    fn FPDFPage_CloseAnnot(&self, annot: FPDF_ANNOTATION);

    /// Removes the annotation in `page` at `index`.
    ///
    ///   `page`  - handle to a page.
    ///
    ///   `index` - the index of the annotation.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFPage_RemoveAnnot(&self, page: FPDF_PAGE, index: c_int) -> FPDF_BOOL;

    /// Gets the subtype of an annotation.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Returns the annotation subtype.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetSubtype(&self, annot: FPDF_ANNOTATION) -> FPDF_ANNOTATION_SUBTYPE;

    /// Checks if an annotation subtype is currently supported for object extraction,
    /// update, and removal.
    ///
    /// Currently supported subtypes: ink and stamp.
    ///
    ///   `subtype`   - the subtype to be checked.
    ///
    /// Returns `true` if this subtype supported.
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsObjectSupportedSubtype(&self, subtype: FPDF_ANNOTATION_SUBTYPE) -> FPDF_BOOL;

    /// Updates `obj` in `annot`. `obj` must be in `annot` already and must have
    /// been retrieved by [PdfiumLibraryBindings::FPDFAnnot_GetObject]. Currently, only ink and stamp
    /// annotations are supported by this API. Also note that only path, image, and
    /// text objects have APIs for modification; see `FPDFPath_*()`, `FPDFText_*()`, and
    /// `FPDFImageObj_*()`.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `obj`    - handle to the object that `annot` needs to update.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_UpdateObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    /// Adds a new InkStroke, represented by an array of points, to the InkList of
    /// `annot`. The API creates an InkList if one doesn't already exist in `annot`.
    /// This API works only for ink annotations. Please refer to ISO 32000-1:2008
    /// spec, section 12.5.6.13.
    ///
    ///   `annot`       - handle to an annotation.
    ///
    ///   `points`      - pointer to a `FS_POINTF` array representing input points.
    ///
    ///   `point_count` - number of elements in `points` array. This should not exceed
    ///                   the maximum value that can be represented by an `int32_t`.
    ///
    /// Returns the 0-based index at which the new InkStroke is added in the InkList
    /// of the `annot`. Returns -1 on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddInkStroke(
        &self,
        annot: FPDF_ANNOTATION,
        points: *const FS_POINTF,
        point_count: size_t,
    ) -> c_int;

    /// Removes an InkList in `annot`.
    ///
    /// This API works only for ink annotations.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Return true on successful removal of `/InkList` entry from context of the
    /// non-null ink `annot`. Returns `false` on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveInkList(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL;

    /// Adds `obj` to `annot`. `obj` must have been created by
    /// [PdfiumLibraryBindings::FPDFPageObj_CreateNewPath], [PdfiumLibraryBindings::FPDFPageObj_CreateNewRect],
    /// [PdfiumLibraryBindings::FPDFPageObj_NewTextObj], or [PdfiumLibraryBindings::FPDFPageObj_NewImageObj], and
    /// will be owned by `annot`. Note that an `obj` cannot belong to more than one
    /// `annot`. Currently, only ink and stamp annotations are supported by this API.
    /// Also note that only path, image, and text objects have APIs for creation.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `obj`    - handle to the object that is to be added to `annot`.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendObject(&self, annot: FPDF_ANNOTATION, obj: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    /// Gets the total number of objects in `annot`, including path objects, text
    /// objects, external objects, image objects, and shading objects.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Returns the number of objects in `annot`.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObjectCount(&self, annot: FPDF_ANNOTATION) -> c_int;

    /// Gets the object in `annot` at `index`.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `index`  - the index of the object.
    ///
    /// Returns a handle to the object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_PAGEOBJECT;

    /// Removes the object in `annot` at `index`.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `index`  - the index of the object to be removed.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_RemoveObject(&self, annot: FPDF_ANNOTATION, index: c_int) -> FPDF_BOOL;

    /// Sets the color of an annotation. Fails when called on annotations with
    /// appearance streams already defined; instead use
    /// [PdfiumLibraryBindings::FPDFPageObj_SetStrokeColor] or [PdfiumLibraryBindings::FPDFPageObj_SetFillColor].
    ///
    ///   `annot`        - handle to an annotation.
    ///
    ///   `type`         - type of the color to be set.
    ///
    ///   `R`, `G`, `B`  - buffers to hold the RGB values of the color. Ranges from 0 to 255.
    ///
    ///   `A`            - buffers to hold the opacity. Ranges from 0 to 255.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetColor(
        &self,
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL;

    /// Gets the color of an annotation. If no color is specified, default to yellow
    /// for highlight annotation, black for all else. Fails when called on
    /// annotations with appearance streams already defined; instead use
    /// [PdfiumLibraryBindings::FPDFPageObj_GetStrokeColor] or [PdfiumLibraryBindings::FPDFPageObj_GetFillColor].
    ///
    ///   `annot`        - handle to an annotation.
    ///
    ///   `type`         - type of the color requested.
    ///
    ///   `R`, `G`, `B`  - buffers to hold the RGB values of the color. Ranges from 0 to 255.
    ///
    ///   `A`            - buffer to hold the opacity. Ranges from 0 to 255.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetColor(
        &self,
        annot: FPDF_ANNOTATION,
        color_type: FPDFANNOT_COLORTYPE,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    /// Checks if the annotation is of a type that has attachment points
    /// (i.e. quadpoints). Quadpoints are the vertices of the rectangle that
    /// encompasses the texts affected by the annotation. They provide the
    /// coordinates in the page where the annotation is attached. Only text markup
    /// annotations (i.e. highlight, strikeout, squiggly, and underline) and link
    /// annotations have quadpoints.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Returns `true` if the annotation is of a type that has quadpoints.
    #[allow(non_snake_case)]
    fn FPDFAnnot_HasAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> FPDF_BOOL;

    /// Replaces the attachment points (i.e. quadpoints) set of an annotation at
    /// `quad_index`. This index needs to be within the result of
    /// [PdfiumLibraryBindings::FPDFAnnot_CountAttachmentPoints].
    ///
    /// If the annotation's appearance stream is defined and this annotation is of a
    /// type with quadpoints, then update the bounding box too if the new quadpoints
    /// define a bigger one.
    ///
    ///   `annot`       - handle to an annotation.
    ///
    ///   `quad_index`  - index of the set of quadpoints.
    ///
    ///   `quad_points` - the quadpoints to be set.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL;

    /// Appends to the list of attachment points (i.e. quadpoints) of an annotation.
    /// If the annotation's appearance stream is defined and this annotation is of a
    /// type with quadpoints, then update the bounding box too if the new quadpoints
    /// define a bigger one.
    ///
    ///   `annot`       - handle to an annotation.
    ///
    ///   `quad_points` - the quadpoints to be set.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_AppendAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_points: *const FS_QUADPOINTSF,
    ) -> FPDF_BOOL;

    /// Gets the number of sets of quadpoints of an annotation.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    /// Returns the number of sets of quadpoints, or 0 on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_CountAttachmentPoints(&self, annot: FPDF_ANNOTATION) -> size_t;

    /// Gets the attachment points (i.e. quadpoints) of an annotation.
    ///
    ///   `annot`       - handle to an annotation.
    ///
    ///   `quad_index`  - index of the set of quadpoints.
    ///
    ///   `quad_points` - receives the quadpoints; must not be `NULL`.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetAttachmentPoints(
        &self,
        annot: FPDF_ANNOTATION,
        quad_index: size_t,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL;

    /// Sets the annotation rectangle defining the location of the annotation. If the
    /// annotation's appearance stream is defined and this annotation is of a type
    /// without quadpoints, then update the bounding box too if the new rectangle
    /// defines a bigger one.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `rect`   - the annotation rectangle to be set.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetRect(&self, annot: FPDF_ANNOTATION, rect: *const FS_RECTF) -> FPDF_BOOL;

    /// Gets the annotation rectangle defining the location of the annotation.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `rect`   - receives the rectangle; must not be `NULL`.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetRect(&self, annot: FPDF_ANNOTATION, rect: *mut FS_RECTF) -> FPDF_BOOL;

    /// Gets the vertices of a polygon or polyline annotation. `buffer` is an array of
    /// points of the annotation. If `length` is less than the returned length, or
    /// `annot` or `buffer` is `NULL`, `buffer` will not be modified.
    ///
    ///   `annot`  - handle to an annotation, as returned by e.g. [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    ///
    ///   `buffer` - buffer for holding the points.
    ///
    ///   `length` - length of the buffer in points.
    ///
    /// Returns the number of points if the annotation is of type polygon or
    /// polyline, 0 otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetVertices(
        &self,
        annot: FPDF_ANNOTATION,
        buffer: *mut FS_POINTF,
        length: c_ulong,
    ) -> c_ulong;

    /// Gets the number of paths in the ink list of an ink annotation.
    ///
    ///   `annot`  - handle to an annotation, as returned by e.g. [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    ///
    /// Returns the number of paths in the ink list if the annotation is of type ink,
    /// 0 otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetInkListCount(&self, annot: FPDF_ANNOTATION) -> c_ulong;

    /// Gets a path in the ink list of an ink annotation. `buffer` is an array of
    /// points of the path. If `length` is less than the returned length, or `annot`
    /// or `buffer` is `NULL`, `buffer` will not be modified.
    ///
    ///   `annot`  - handle to an annotation, as returned by e.g. [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    ///
    ///   `path_index` - index of the path.
    ///
    ///   `buffer` - buffer for holding the points.
    ///
    ///   `length` - length of the buffer in points.
    ///
    /// Returns the number of points of the path if the annotation is of type ink, 0
    /// otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetInkListPath(
        &self,
        annot: FPDF_ANNOTATION,
        path_index: c_ulong,
        buffer: *mut FS_POINTF,
        length: c_ulong,
    ) -> c_ulong;

    /// Gets the starting and ending coordinates of a line annotation.
    ///
    ///   `annot`  - handle to an annotation, as returned by e.g. [PdfiumLibraryBindings::FPDFPage_GetAnnot]
    ///
    ///   `start` - starting point
    ///
    ///   `end` - ending point
    ///
    /// Returns `true` if the annotation is of type line and `start` and `end` are not
    /// `NULL`, `false` otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLine(
        &self,
        annot: FPDF_ANNOTATION,
        start: *mut FS_POINTF,
        end: *mut FS_POINTF,
    ) -> FPDF_BOOL;

    /// Sets the characteristics of the annotation's border (rounded rectangle).
    ///
    ///   `annot`              - handle to an annotation.
    ///
    ///   `horizontal_radius`  - horizontal corner radius, in default user space units.
    ///
    ///   `vertical_radius`    - vertical corner radius, in default user space units.
    ///
    ///   `border_width`       - border width, in default user space units.
    ///
    /// Returns `true` if setting the border for `annot` succeeds, `false` otherwise.
    ///
    /// If `annot` contains an appearance stream that overrides the border values,
    /// then the appearance stream will be removed on success.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetBorder(
        &self,
        annot: FPDF_ANNOTATION,
        horizontal_radius: c_float,
        vertical_radius: c_float,
        border_width: c_float,
    ) -> FPDF_BOOL;

    /// Gets the characteristics of the annotation's border (rounded rectangle).
    ///
    ///   `annot`              - handle to an annotation.
    ///
    ///   `horizontal_radius`  - horizontal corner radius, in default user space units.
    ///
    ///   `vertical_radius`    - vertical corner radius, in default user space units.
    ///
    ///   `border_width`       - border width, in default user space units.
    ///
    /// Returns `true` if `horizontal_radius`, `vertical_radius` and `border_width` are
    /// not `NULL`, `false` otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetBorder(
        &self,
        annot: FPDF_ANNOTATION,
        horizontal_radius: *mut c_float,
        vertical_radius: *mut c_float,
        border_width: *mut c_float,
    ) -> FPDF_BOOL;

    /// Get the JavaScript of an event of the annotation's additional actions.
    ///
    /// `buffer` is only modified if `buflen` is large enough to hold the whole
    /// JavaScript string. If `buflen` is smaller, the total size of the JavaScript
    /// is still returned, but nothing is copied.  If there is no JavaScript for
    /// `event` in `annot`, an empty string is written to `buf` and 2 is returned,
    /// denoting the size of the null terminator in the buffer. On other errors,
    /// nothing is written to `buffer` and 0 is returned.
    ///
    ///   `hHandle`     -   handle to the form fill module, returned by
    ///                     [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment()].
    ///
    ///   `annot`       -   handle to an interactive form annotation.
    ///
    ///   `event`       -   event type, one of the `FPDF_ANNOT_AACTION_*` values.
    ///
    ///   `buffer`      -   buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///   `buflen`     -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes, including the 2-byte null terminator.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormAdditionalActionJavaScript(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        event: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the alternate name of `annot`, which is an interactive form annotation.
    ///
    /// `buffer` is only modified if `buflen` is longer than the length of contents.
    /// In case of error, nothing will be added to `buffer` and the return value will be 0.
    /// Note that return value of empty string is 2 for `\0\0`.
    ///
    ///   `hHandle`     -   handle to the form fill module, returned by
    ///                     [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment()].
    ///
    ///   `annot`       -   handle to an interactive form annotation.
    ///
    ///   `buffer`      -   buffer for holding the alternate name string, encoded in
    ///                     UTF-16LE.
    ///
    ///   `buflen`     -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldAlternateName(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Check if `annot`'s dictionary has `key` as a key.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to look for, encoded in UTF-8.
    ///
    /// Returns `true` if `key` exists.
    #[allow(non_snake_case)]
    fn FPDFAnnot_HasKey(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_BOOL;

    /// Gets the type of the value corresponding to `key` in `annot`'s dictionary.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to look for, encoded in UTF-8.
    ///
    /// Returns the type of the dictionary value.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetValueType(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_OBJECT_TYPE;

    /// Sets the string value corresponding to `key` in `annot`'s dictionary,
    /// overwriting the existing value if any. The value type would be
    /// `FPDF_OBJECT_STRING` after this function call succeeds.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the dictionary entry to be set, encoded in UTF-8.
    ///
    ///   `value`  - the string value to be set, encoded in UTF-16LE.
    ///
    /// Returns `true` if successful.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFAnnot_SetStringValue_str].
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetStringValue(
        &self,
        annot: FPDF_ANNOTATION,
        key: &str,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFAnnot_SetStringValue].
    ///
    /// Sets the string value corresponding to `key` in `annot`'s dictionary,
    /// overwriting the existing value if any. The value type would be
    /// `FPDF_OBJECT_STRING` after this function call succeeds.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the dictionary entry to be set.
    ///
    ///   `value`  - the string value to be set.
    ///
    /// Returns `true` if successful.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetStringValue_str(
        &self,
        annot: FPDF_ANNOTATION,
        key: &str,
        value: &str,
    ) -> FPDF_BOOL {
        self.FPDFAnnot_SetStringValue(
            annot,
            key,
            get_pdfium_utf16le_bytes_from_str(value).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Gets the string value corresponding to `key` in `annot`'s dictionary. `buffer`
    /// is only modified if `buflen` is longer than the length of contents. Note that
    /// if `key` does not exist in the dictionary or if `key`'s corresponding value
    /// in the dictionary is not a string (i.e. the value is not of type
    /// `FPDF_OBJECT_STRING` or `FPDF_OBJECT_NAME`), then an empty string would be copied
    /// to `buffer` and the return value would be 2. On other errors, nothing would
    /// be added to `buffer` and the return value would be 0.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the requested dictionary entry, encoded in UTF-8.
    ///
    ///   `buffer` - buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///   `buflen` - length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetStringValue(
        &self,
        annot: FPDF_ANNOTATION,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the float value corresponding to `key` in `annot`'s dictionary. Writes
    /// value to `value` and returns `true` if `key` exists in the dictionary and
    /// `key`'s corresponding value is a number (`FPDF_OBJECT_NUMBER`), `false`
    /// otherwise.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the requested dictionary entry, encoded in UTF-8.
    ///
    ///   `value`  - receives the value, must not be `NULL`.
    ///
    /// Returns `true` if value found, `false` otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetNumberValue(
        &self,
        annot: FPDF_ANNOTATION,
        key: &str,
        value: *mut c_float,
    ) -> FPDF_BOOL;

    /// Sets the AP (appearance string) in `annot`'s dictionary for a given
    /// `appearanceMode`.
    ///
    ///   `annot`          - handle to an annotation.
    ///
    ///   `appearanceMode` - the appearance mode (normal, rollover or down) for which
    ///                      to set the AP.
    ///
    ///   `value`          - the string value to be set, encoded in UTF-16LE. If
    ///                      `nullptr` is passed, the AP is cleared for that mode. If the
    ///                      mode is Normal, APs for all modes are cleared.
    ///
    /// Returns `true` if successful.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFAnnot_SetAP_str].
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAP(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFAnnot_SetAP].
    ///
    /// Sets the AP (appearance string) in `annot`'s dictionary for a given
    /// `appearanceMode`.
    ///
    ///   `annot`          - handle to an annotation.
    ///
    ///   `appearanceMode` - the appearance mode (normal, rollover or down) for which
    ///                      to set the AP.
    ///
    ///   `value`          - the string value to be set.
    ///
    /// Returns `true` if successful.
    ///
    /// Note that this helper function cannot clear appearance strings, since it cannot pass
    /// a null pointer for `value`. To clear an appearance string, use [PdfiumLibraryBindings::FPDFAnnot_SetAP].
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetAP_str(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        value: &str,
    ) -> FPDF_BOOL {
        self.FPDFAnnot_SetAP(
            annot,
            appearanceMode,
            get_pdfium_utf16le_bytes_from_str(value).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Gets the AP (appearance string) from `annot`'s dictionary for a given
    /// `appearanceMode`.
    ///
    /// `buffer` is only modified if `buflen` is large enough to hold the whole AP
    /// string. If `buflen` is smaller, the total size of the AP is still returned,
    /// but nothing is copied.
    ///
    /// If there is no appearance stream for `annot` in `appearanceMode`, an empty
    /// string is written to `buf` and 2 is returned.
    ///
    /// On other errors, nothing is written to `buffer` and 0 is returned.
    ///
    ///   `annot`          - handle to an annotation.
    ///
    ///   `appearanceMode` - the appearance mode (normal, rollover or down) for which
    ///                      to get the AP.
    ///
    ///   `buffer`         - buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///   `buflen`         - length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetAP(
        &self,
        annot: FPDF_ANNOTATION,
        appearanceMode: FPDF_ANNOT_APPEARANCEMODE,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the annotation corresponding to `key` in `annot`'s dictionary. Common
    /// keys for linking annotations include "IRT" and "Popup". Must call
    /// [PdfiumLibraryBindings::FPDFPage_CloseAnnot] when the annotation returned by this function
    /// is no longer needed.
    ///
    ///   `annot`  - handle to an annotation.
    ///
    ///   `key`    - the key to the requested dictionary entry, encoded in UTF-8.
    ///
    /// Returns a handle to the linked annotation object, or `NULL` on failure.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLinkedAnnot(&self, annot: FPDF_ANNOTATION, key: &str) -> FPDF_ANNOTATION;

    /// Gets the annotation flags of `annot`.
    ///
    ///   `annot`    - handle to an annotation.
    ///
    /// Returns the annotation flags.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFlags(&self, annot: FPDF_ANNOTATION) -> c_int;

    /// Sets the `annot`'s flags to be of the value `flags`.
    ///
    ///   `annot`      - handle to an annotation.
    ///
    ///   `flags`      - the flag values to be set.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFlags(&self, annot: FPDF_ANNOTATION, flags: c_int) -> FPDF_BOOL;

    /// Gets the annotation flags of `annot`.
    ///
    ///    `hHandle`    -   handle to the form fill module, returned by
    ///                     [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///    `annot`      -   handle to an interactive form annotation.
    ///
    /// Returns the annotation flags specific to interactive forms.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldFlags(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
    ) -> c_int;

    /// Retrieves an interactive form annotation whose rectangle contains a given
    /// point on a page. Must call [PdfiumLibraryBindings::FPDFPage_CloseAnnot] when the
    /// annotation returned is no longer needed.
    ///
    ///    `hHandle`    -   handle to the form fill module, returned by
    ///                     [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `page`       -   handle to the page, returned by [PdfiumLibraryBindings::FPDF_LoadPage] function.
    ///
    ///    `point`      -   position in PDF "user space".
    ///
    /// Returns the interactive form annotation whose rectangle contains the given
    /// coordinates on the page. If there is no such annotation, return `NULL`.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldAtPoint(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        point: *const FS_POINTF,
    ) -> FPDF_ANNOTATION;

    /// Gets the name of `annot`, which is an interactive form annotation.
    /// `buffer` is only modified if `buflen` is longer than the length of contents.
    /// In case of error, nothing will be added to `buffer` and the return value will
    /// be 0. Note that return value of empty string is 2 for "\0\0".
    ///
    ///    `hHandle`     -   handle to the form fill module, returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`       -   handle to an interactive form annotation.
    ///
    ///    `buffer`      -   buffer for holding the name string, encoded in UTF-16LE.
    ///
    ///    `buflen`      -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldName(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the form field type of `annot`, which is an interactive form annotation.
    ///
    ///    `hHandle`     -   handle to the form fill module, returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`       -   handle to an interactive form annotation.
    ///
    /// Returns the type of the form field (one of the `FPDF_FORMFIELD_*` values) on
    /// success. Returns -1 on error.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldType(&self, hHandle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION)
        -> c_int;

    /// Gets the value of `annot`, which is an interactive form annotation.
    /// `buffer` is only modified if `buflen` is longer than the length of contents.
    /// In case of error, nothing will be added to `buffer` and the return value will
    /// be 0. Note that return value of empty string is 2 for "\0\0".
    ///
    ///    `hHandle`     -   handle to the form fill module, returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`       -   handle to an interactive form annotation.
    ///
    ///    `buffer`      -   buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///    `buflen`      -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldValue(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the number of options in the `annot`'s "Opt" dictionary. Intended for
    /// use with listbox and combobox widget annotations.
    ///
    ///   `hHandle` - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns the number of options in "Opt" dictionary on success. Return value
    /// will be -1 if annotation does not have an "Opt" dictionary or other error.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetOptionCount(&self, hHandle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> c_int;

    /// Gets the string value for the label of the option at `index` in `annot`'s
    /// "Opt" dictionary. Intended for use with listbox and combobox widget
    /// annotations. `buffer` is only modified if `buflen` is longer than the length
    /// of contents. If index is out of range or in case of other error, nothing
    /// will be added to `buffer` and the return value will be 0. Note that
    /// return value of empty string is 2 for "\0\0".
    ///
    ///   `hHandle` - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    ///   `index`   - numeric index of the option in the "Opt" array.
    ///
    ///   `buffer`  - buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///   `buflen`  - length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    /// If `annot` does not have an "Opt" array, `index` is out of range or if any
    /// other error occurs, returns 0.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetOptionLabel(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        index: c_int,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Determines whether or not the option at `index` in `annot`'s "Opt" dictionary
    /// is selected. Intended for use with listbox and combobox widget annotations.
    ///
    ///   `hHandle` - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    ///   `index`   - numeric index of the option in the "Opt" array.
    ///
    /// Returns `true` if the option at `index` in `annot`'s "Opt" dictionary is
    /// selected, `false` otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsOptionSelected(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        index: c_int,
    ) -> FPDF_BOOL;

    /// Gets the float value of the font size for an `annot` with variable text.
    /// If 0, the font is to be auto-sized: its size is computed as a function of
    /// the height of the annotation rectangle.
    ///
    ///   `hHandle` - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    ///   `value`   - Required. Float which will be set to font size on success.
    ///
    /// Returns `true` if the font size was set in `value`, `false` on error or if
    /// `value` not provided.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFontSize(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        value: *mut c_float,
    ) -> FPDF_BOOL;

    #[cfg(any(
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Gets the RGB value of the font color for an `annot` with variable text.
    ///
    ///   `hHandle`  - handle to the form fill module, returned by
    ///                [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`    - handle to an annotation.
    ///
    ///   `R`, `G`, `B`  - buffer to hold the RGB value of the color. Ranges from 0 to 255.
    ///
    /// Returns `true` if the font color was set, `false` on error or if the font color
    /// was not provided.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFontColor(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
    ) -> FPDF_BOOL;

    /// Determines if `annot` is a form widget that is checked. Intended for use with
    /// checkbox and radio button widgets.
    ///
    ///   `hHandle` - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns `true` if `annot` is a form widget and is checked, `false` otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_IsChecked(&self, hHandle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL;

    /// Sets the list of focusable annotation subtypes. Annotations of subtype
    /// `FPDF_ANNOT_WIDGET` are by default focusable. New subtypes set using this API
    /// will override the existing subtypes.
    ///
    ///   `hHandle`  - handle to the form fill module, returned by
    ///                [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `subtypes` - list of annotation subtype which can be tabbed over.
    ///
    ///   `count`    - total number of annotation subtype in list.
    ///
    /// Returns `true` if list of annotation subtype is set successfully, `false` otherwise.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetFocusableSubtypes(
        &self,
        hHandle: FPDF_FORMHANDLE,
        subtypes: *const FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL;

    /// Gets the count of focusable annotation subtypes as set by host
    /// for a `hHandle`.
    ///
    ///   `hHandle`  - handle to the form fill module, returned by
    ///                [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    /// Returns the count of focusable annotation subtypes or -1 on error.
    ///
    /// Note: Annotations of type `FPDF_ANNOT_WIDGET` are by default focusable.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypesCount(&self, hHandle: FPDF_FORMHANDLE) -> c_int;

    /// Gets the list of focusable annotation subtype as set by host.
    ///
    ///   `hHandle`  - handle to the form fill module, returned by
    ///                [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `subtypes` - receives the list of annotation subtype which can be tabbed
    ///                over. Caller must have allocated `subtypes` more than or
    ///                equal to the count obtained from
    ///                [PdfiumLibraryBindings::FPDFAnnot_GetFocusableSubtypesCount] API.
    ///
    ///   `count`    - size of `subtypes`.
    ///
    /// Returns `true` on success and set list of annotation subtype to `subtypes`,
    /// `false` otherwise.
    ///
    /// Note: Annotations of type `FPDF_ANNOT_WIDGET` are by default focusable.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFocusableSubtypes(
        &self,
        hHandle: FPDF_FORMHANDLE,
        subtypes: *mut FPDF_ANNOTATION_SUBTYPE,
        count: size_t,
    ) -> FPDF_BOOL;

    /// Gets `FPDF_LINK` object for `annot`. Intended to use for link annotations.
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns `FPDF_LINK` from the `FPDF_ANNOTATION` and `NULL` on failure,
    /// if the input annot is `NULL`, or input annot's subtype is not link.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetLink(&self, annot: FPDF_ANNOTATION) -> FPDF_LINK;

    /// Gets the count of annotations in the `annot`'s control group.
    ///
    /// A group of interactive form annotations is collectively called a form
    /// control group. Here, `annot`, an interactive form annotation, should be
    /// either a radio button or a checkbox.
    ///
    ///   `hHandle` - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns number of controls in its control group or -1 on error.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlCount(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
    ) -> c_int;

    /// Gets the index of `annot` in `annot`'s control group.
    ///
    /// A group of interactive form annotations is collectively called a form
    /// control group. Here, `annot`, an interactive form annotation, should be
    /// either a radio button or a checkbox.
    ///
    ///   `hHandle` - handle to the form fill module, returned by
    ///               [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///   `annot`   - handle to an annotation.
    ///
    /// Returns index of a given `annot` in its control group or -1 on error.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormControlIndex(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
    ) -> c_int;

    /// Gets the export value of `annot` which is an interactive form annotation.
    ///
    /// Intended for use with radio button and checkbox widget annotations.
    ///
    /// `buffer` is only modified if `buflen` is longer than the length of contents.
    /// In case of error, nothing will be added to `buffer` and the return value
    /// will be 0. Note that return value of empty string is 2 for "\0\0".
    ///
    ///    `hHandle`     -   handle to the form fill module, returned by
    ///                      [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    ///    `annot`       -   handle to an interactive form annotation.
    ///
    ///    `buffer`      -   buffer for holding the value string, encoded in UTF-16LE.
    ///
    ///    `buflen`      -   length of the buffer in bytes.
    ///
    /// Returns the length of the string value in bytes.
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFormFieldExportValue(
        &self,
        hHandle: FPDF_FORMHANDLE,
        annot: FPDF_ANNOTATION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Add a URI action to `annot`, overwriting the existing action, if any.
    ///
    ///   `annot`  - handle to a link annotation.
    ///
    ///   `uri`    - the URI to be set, encoded in 7-bit ASCII.
    ///
    /// Returns `true` if successful.
    #[allow(non_snake_case)]
    fn FPDFAnnot_SetURI(&self, annot: FPDF_ANNOTATION, uri: &str) -> FPDF_BOOL;

    /// Get the attachment from `annot`.
    ///
    ///   `annot`  - handle to a file annotation.
    ///
    /// Returns the handle to the attachment object, or NULL on failure.
    #[cfg(any(
        feature = "pdfium_6337",
        feature = "pdfium_6406",
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[allow(non_snake_case)]
    fn FPDFAnnot_GetFileAttachment(&self, annot: FPDF_ANNOTATION) -> FPDF_ATTACHMENT;

    /// Add an embedded file with `name` to `annot`.
    ///
    ///   `annot`    - handle to a file annotation.
    ///
    ///   `name`     - name of the new attachment.
    ///
    /// Returns a handle to the new attachment object, or NULL on failure.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFAnnot_AddFileAttachment_str].
    #[cfg(any(
        feature = "pdfium_6337",
        feature = "pdfium_6406",
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddFileAttachment(
        &self,
        annot: FPDF_ANNOTATION,
        name: FPDF_WIDESTRING,
    ) -> FPDF_ATTACHMENT;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFAnnot_AddFileAttachment].
    ///
    /// Add an embedded file with `name` to `annot`.
    ///
    ///   `annot`    - handle to a file annotation.
    ///
    ///   `name`     - name of the new attachment.
    ///
    /// Returns a handle to the new attachment object, or NULL on failure.
    #[cfg(any(
        feature = "pdfium_6337",
        feature = "pdfium_6406",
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAnnot_AddFileAttachment_str(
        &self,
        annot: FPDF_ANNOTATION,
        name: &str,
    ) -> FPDF_ATTACHMENT {
        self.FPDFAnnot_AddFileAttachment(
            annot,
            get_pdfium_utf16le_bytes_from_str(name).as_ptr() as FPDF_WIDESTRING,
        )
    }

    ///  Initializes the form fill environment.
    ///
    ///    `document` - Handle to document from [PdfiumLibraryBindings::FPDF_LoadDocument].
    ///
    ///    `formInfo` - Pointer to a `FPDF_FORMFILLINFO` structure.
    ///
    /// Return Value:
    ///        Handle to the form fill module, or `NULL` on failure.
    ///
    /// Comments:
    ///        This function should be called before any form fill operation.
    ///        The `FPDF_FORMFILLINFO` passed in via `form_info` must remain valid until
    ///        the returned `FPDF_FORMHANDLE` is closed.
    #[allow(non_snake_case)]
    fn FPDFDOC_InitFormFillEnvironment(
        &self,
        document: FPDF_DOCUMENT,
        form_info: *mut FPDF_FORMFILLINFO,
    ) -> FPDF_FORMHANDLE;

    /// Takes ownership of `hHandle` and exits the form fill environment.
    ///
    ///    `hHandle`  -   Handle to the form fill module, as returned by
    ///                   [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    ///
    /// This function is a no-op when `hHandle` is null.
    #[allow(non_snake_case)]
    fn FPDFDOC_ExitFormFillEnvironment(&self, hHandle: FPDF_FORMHANDLE);

    /// This method is required for implementing all the form related
    /// functions. Should be invoked after user successfully loaded a
    /// PDF page, and [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment] has been invoked.
    ///
    ///    `hHandle`   -   Handle to the form fill module, as returned by
    ///                    [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    #[allow(non_snake_case)]
    fn FORM_OnAfterLoadPage(&self, page: FPDF_PAGE, hHandle: FPDF_FORMHANDLE);

    /// This method is required for implementing all the form related
    /// functions. Should be invoked before user closes the PDF page.
    ///
    ///    `page`      -   Handle to the page, as returned by [PdfiumLibraryBindings::FPDF_LoadPage].
    ///
    ///    `hHandle`   -   Handle to the form fill module, as returned by
    ///                    [PdfiumLibraryBindings::FPDFDOC_InitFormFillEnvironment].
    #[allow(non_snake_case)]
    fn FORM_OnBeforeClosePage(&self, page: FPDF_PAGE, hHandle: FPDF_FORMHANDLE);

    #[allow(non_snake_case)]
    fn FPDFDoc_GetPageMode(&self, document: FPDF_DOCUMENT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPage_Flatten(&self, page: FPDF_PAGE, nFlag: c_int) -> c_int;

    #[doc = " Function: FORM_DoDocumentJSAction\n       This method is required for performing document-level JavaScript\n       actions. It should be invoked after the PDF document has been loaded.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n Return Value:\n       None.\n Comments:\n       If there is document-level JavaScript action embedded in the\n       document, this method will execute the JavaScript action. Otherwise,\n       the method will do nothing."]
    #[allow(non_snake_case)]
    fn FORM_DoDocumentJSAction(&self, hHandle: FPDF_FORMHANDLE);

    #[doc = " Function: FORM_DoDocumentOpenAction\n       This method is required for performing open-action when the document\n       is opened.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n Return Value:\n       None.\n Comments:\n       This method will do nothing if there are no open-actions embedded\n       in the document."]
    #[allow(non_snake_case)]
    fn FORM_DoDocumentOpenAction(&self, hHandle: FPDF_FORMHANDLE);

    #[doc = " Function: FORM_DoDocumentAAction\n       This method is required for performing the document's\n       additional-action.\n Parameters:\n       hHandle     -   Handle to the form fill module. Returned by\n                       FPDFDOC_InitFormFillEnvironment.\n       aaType      -   The type of the additional-actions which defined\n                       above.\n Return Value:\n       None.\n Comments:\n       This method will do nothing if there is no document\n       additional-action corresponding to the specified |aaType|."]
    #[allow(non_snake_case)]
    fn FORM_DoDocumentAAction(&self, hHandle: FPDF_FORMHANDLE, aaType: c_int);

    #[doc = " Function: FORM_DoPageAAction\n       This method is required for performing the page object's\n       additional-action when opened or closed.\n Parameters:\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       aaType      -   The type of the page object's additional-actions\n                       which defined above.\n Return Value:\n       None.\n Comments:\n       This method will do nothing if no additional-action corresponding\n       to the specified |aaType| exists."]
    #[allow(non_snake_case)]
    fn FORM_DoPageAAction(&self, page: FPDF_PAGE, hHandle: FPDF_FORMHANDLE, aaType: c_int);

    #[doc = " Function: FORM_OnMouseMove\n       Call this member function when the mouse cursor moves.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       modifier    -   Indicates whether various virtual keys are down.\n       page_x      -   Specifies the x-coordinate of the cursor in PDF user\n                       space.\n       page_y      -   Specifies the y-coordinate of the cursor in PDF user\n                       space.\n Return Value:\n       True indicates success; otherwise false."]
    #[allow(non_snake_case)]
    fn FORM_OnMouseMove(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API\n Function: FORM_OnMouseWheel\n       Call this member function when the user scrolls the mouse wheel.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       modifier    -   Indicates whether various virtual keys are down.\n       page_coord  -   Specifies the coordinates of the cursor in PDF user\n                       space.\n       delta_x     -   Specifies the amount of wheel movement on the x-axis,\n                       in units of platform-agnostic wheel deltas. Negative\n                       values mean left.\n       delta_y     -   Specifies the amount of wheel movement on the y-axis,\n                       in units of platform-agnostic wheel deltas. Negative\n                       values mean down.\n Return Value:\n       True indicates success; otherwise false.\n Comments:\n       For |delta_x| and |delta_y|, the caller must normalize\n       platform-specific wheel deltas. e.g. On Windows, a delta value of 240\n       for a WM_MOUSEWHEEL event normalizes to 2, since Windows defines\n       WHEEL_DELTA as 120."]
    #[allow(non_snake_case)]
    fn FORM_OnMouseWheel(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_coord: *const FS_POINTF,
        delta_x: c_int,
        delta_y: c_int,
    ) -> FPDF_BOOL;

    #[doc = " Function: FORM_OnFocus\n       This function focuses the form annotation at a given point. If the\n       annotation at the point already has focus, nothing happens. If there\n       is no annotation at the point, removes form focus.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       modifier    -   Indicates whether various virtual keys are down.\n       page_x      -   Specifies the x-coordinate of the cursor in PDF user\n                       space.\n       page_y      -   Specifies the y-coordinate of the cursor in PDF user\n                       space.\n Return Value:\n       True if there is an annotation at the given point and it has focus."]
    #[allow(non_snake_case)]
    fn FORM_OnFocus(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    #[doc = " Function: FORM_OnLButtonDown\n       Call this member function when the user presses the left\n       mouse button.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       modifier    -   Indicates whether various virtual keys are down.\n       page_x      -   Specifies the x-coordinate of the cursor in PDF user\n                       space.\n       page_y      -   Specifies the y-coordinate of the cursor in PDF user\n                       space.\n Return Value:\n       True indicates success; otherwise false."]
    #[allow(non_snake_case)]
    fn FORM_OnLButtonDown(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    #[doc = " Function: FORM_OnRButtonDown\n       Same as above, execpt for the right mouse button.\n Comments:\n       At the present time, has no effect except in XFA builds, but is\n       included for the sake of symmetry."]
    #[allow(non_snake_case)]
    fn FORM_OnRButtonDown(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    #[doc = " Function: FORM_OnLButtonUp\n       Call this member function when the user releases the left\n       mouse button.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       modifier    -   Indicates whether various virtual keys are down.\n       page_x      -   Specifies the x-coordinate of the cursor in device.\n       page_y      -   Specifies the y-coordinate of the cursor in device.\n Return Value:\n       True indicates success; otherwise false."]
    #[allow(non_snake_case)]
    fn FORM_OnLButtonUp(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    #[doc = " Function: FORM_OnRButtonUp\n       Same as above, execpt for the right mouse button.\n Comments:\n       At the present time, has no effect except in XFA builds, but is\n       included for the sake of symmetry."]
    #[allow(non_snake_case)]
    fn FORM_OnRButtonUp(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    #[doc = " Function: FORM_OnLButtonDoubleClick\n       Call this member function when the user double clicks the\n       left mouse button.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       modifier    -   Indicates whether various virtual keys are down.\n       page_x      -   Specifies the x-coordinate of the cursor in PDF user\n                       space.\n       page_y      -   Specifies the y-coordinate of the cursor in PDF user\n                       space.\n Return Value:\n       True indicates success; otherwise false."]
    #[allow(non_snake_case)]
    fn FORM_OnLButtonDoubleClick(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        modifier: c_int,
        page_x: f64,
        page_y: f64,
    ) -> FPDF_BOOL;

    #[doc = " Function: FORM_OnKeyDown\n       Call this member function when a nonsystem key is pressed.\n Parameters:\n       hHandle     -   Handle to the form fill module, aseturned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       nKeyCode    -   The virtual-key code of the given key (see\n                       fpdf_fwlevent.h for virtual key codes).\n       modifier    -   Mask of key flags (see fpdf_fwlevent.h for key\n                       flag values).\n Return Value:\n       True indicates success; otherwise false."]
    #[allow(non_snake_case)]
    fn FORM_OnKeyDown(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        nKeyCode: c_int,
        modifier: c_int,
    ) -> FPDF_BOOL;

    #[doc = " Function: FORM_OnKeyUp\n       Call this member function when a nonsystem key is released.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       nKeyCode    -   The virtual-key code of the given key (see\n                       fpdf_fwlevent.h for virtual key codes).\n       modifier    -   Mask of key flags (see fpdf_fwlevent.h for key\n                       flag values).\n Return Value:\n       True indicates success; otherwise false.\n Comments:\n       Currently unimplemented and always returns false. PDFium reserves this\n       API and may implement it in the future on an as-needed basis."]
    #[allow(non_snake_case)]
    fn FORM_OnKeyUp(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        nKeyCode: c_int,
        modifier: c_int,
    ) -> FPDF_BOOL;

    #[doc = " Function: FORM_OnChar\n       Call this member function when a keystroke translates to a\n       nonsystem character.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       nChar       -   The character code value itself.\n       modifier    -   Mask of key flags (see fpdf_fwlevent.h for key\n                       flag values).\n Return Value:\n       True indicates success; otherwise false."]
    #[allow(non_snake_case)]
    fn FORM_OnChar(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        nChar: c_int,
        modifier: c_int,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API\n Function: FORM_GetFocusedText\n       Call this function to obtain the text within the current focused\n       field, if any.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       buffer      -   Buffer for holding the form text, encoded in\n                       UTF-16LE. If NULL, |buffer| is not modified.\n       buflen      -   Length of |buffer| in bytes. If |buflen| is less\n                       than the length of the form text string, |buffer| is\n                       not modified.\n Return Value:\n       Length in bytes for the text in the focused field."]
    #[allow(non_snake_case)]
    fn FORM_GetFocusedText(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    #[doc = " Function: FORM_GetSelectedText\n       Call this function to obtain selected text within a form text\n       field or form combobox text field.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n       buffer      -   Buffer for holding the selected text, encoded in\n                       UTF-16LE. If NULL, |buffer| is not modified.\n       buflen      -   Length of |buffer| in bytes. If |buflen| is less\n                       than the length of the selected text string,\n                       |buffer| is not modified.\n Return Value:\n       Length in bytes of selected text in form text field or form combobox\n       text field."]
    #[allow(non_snake_case)]
    fn FORM_GetSelectedText(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    #[doc = " Experimental API\n Function: FORM_ReplaceAndKeepSelection\n       Call this function to replace the selected text in a form\n       text field or user-editable form combobox text field with another\n       text string (which can be empty or non-empty). If there is no\n       selected text, this function will append the replacement text after\n       the current caret position. After the insertion, the inserted text\n       will be selected.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as Returned by FPDF_LoadPage().\n       wsText      -   The text to be inserted, in UTF-16LE format.\n Return Value:\n       None."]
    #[allow(non_snake_case)]
    fn FORM_ReplaceAndKeepSelection(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        wsText: FPDF_WIDESTRING,
    );

    #[doc = " Function: FORM_ReplaceSelection\n       Call this function to replace the selected text in a form\n       text field or user-editable form combobox text field with another\n       text string (which can be empty or non-empty). If there is no\n       selected text, this function will append the replacement text after\n       the current caret position. After the insertion, the selection range\n       will be set to empty.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as Returned by FPDF_LoadPage().\n       wsText      -   The text to be inserted, in UTF-16LE format.\n Return Value:\n       None."]
    #[allow(non_snake_case)]
    fn FORM_ReplaceSelection(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        wsText: FPDF_WIDESTRING,
    );

    #[doc = " Experimental API\n Function: FORM_SelectAllText\n       Call this function to select all the text within the currently focused\n       form text field or form combobox text field.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n Return Value:\n       Whether the operation succeeded or not."]
    #[allow(non_snake_case)]
    fn FORM_SelectAllText(&self, hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    #[doc = " Function: FORM_CanUndo\n       Find out if it is possible for the current focused widget in a given\n       form to perform an undo operation.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n Return Value:\n       True if it is possible to undo."]
    #[allow(non_snake_case)]
    fn FORM_CanUndo(&self, hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    #[doc = " Function: FORM_CanRedo\n       Find out if it is possible for the current focused widget in a given\n       form to perform a redo operation.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n Return Value:\n       True if it is possible to redo."]
    #[allow(non_snake_case)]
    fn FORM_CanRedo(&self, hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    #[doc = " Function: FORM_Undo\n       Make the current focused widget perform an undo operation.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n Return Value:\n       True if the undo operation succeeded."]
    #[allow(non_snake_case)]
    fn FORM_Undo(&self, hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    #[doc = " Function: FORM_Redo\n       Make the current focused widget perform a redo operation.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page        -   Handle to the page, as returned by FPDF_LoadPage().\n Return Value:\n       True if the redo operation succeeded."]
    #[allow(non_snake_case)]
    fn FORM_Redo(&self, hHandle: FPDF_FORMHANDLE, page: FPDF_PAGE) -> FPDF_BOOL;

    #[doc = " Function: FORM_ForceToKillFocus.\n       Call this member function to force to kill the focus of the form\n       field which has focus. If it would kill the focus of a form field,\n       save the value of form field if was changed by theuser.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n Return Value:\n       True indicates success; otherwise false."]
    #[allow(non_snake_case)]
    fn FORM_ForceToKillFocus(&self, hHandle: FPDF_FORMHANDLE) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FORM_GetFocusedAnnot.\n       Call this member function to get the currently focused annotation.\n Parameters:\n       handle      -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       page_index  -   Buffer to hold the index number of the page which\n                       contains the focused annotation. 0 for the first page.\n                       Can't be NULL.\n       annot       -   Buffer to hold the focused annotation. Can't be NULL.\n Return Value:\n       On success, return true and write to the out parameters. Otherwise\n       return false and leave the out parameters unmodified.\n Comments:\n       Not currently supported for XFA forms - will report no focused\n       annotation.\n       Must call FPDFPage_CloseAnnot() when the annotation returned in |annot|\n       by this function is no longer needed.\n       This will return true and set |page_index| to -1 and |annot| to NULL,\n       if there is no focused annotation."]
    #[allow(non_snake_case)]
    fn FORM_GetFocusedAnnot(
        &self,
        handle: FPDF_FORMHANDLE,
        page_index: *mut c_int,
        annot: *mut FPDF_ANNOTATION,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FORM_SetFocusedAnnot.\n       Call this member function to set the currently focused annotation.\n Parameters:\n       handle      -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n       annot       -   Handle to an annotation.\n Return Value:\n       True indicates success; otherwise false.\n Comments:\n       |annot| can't be NULL. To kill focus, use FORM_ForceToKillFocus()\n       instead."]
    #[allow(non_snake_case)]
    fn FORM_SetFocusedAnnot(&self, handle: FPDF_FORMHANDLE, annot: FPDF_ANNOTATION) -> FPDF_BOOL;

    #[doc = " Function: FPDFPage_HasFormFieldAtPoint\n     Get the form field type by point.\n Parameters:\n     hHandle     -   Handle to the form fill module. Returned by\n                     FPDFDOC_InitFormFillEnvironment().\n     page        -   Handle to the page. Returned by FPDF_LoadPage().\n     page_x      -   X position in PDF \"user space\".\n     page_y      -   Y position in PDF \"user space\".\n Return Value:\n     Return the type of the form field; -1 indicates no field.\n     See field types above."]
    #[allow(non_snake_case)]
    fn FPDFPage_HasFormFieldAtPoint(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        page_x: f64,
        page_y: f64,
    ) -> c_int;

    #[doc = " Function: FPDFPage_FormFieldZOrderAtPoint\n     Get the form field z-order by point.\n Parameters:\n     hHandle     -   Handle to the form fill module. Returned by\n                     FPDFDOC_InitFormFillEnvironment().\n     page        -   Handle to the page. Returned by FPDF_LoadPage().\n     page_x      -   X position in PDF \"user space\".\n     page_y      -   Y position in PDF \"user space\".\n Return Value:\n     Return the z-order of the form field; -1 indicates no field.\n     Higher numbers are closer to the front."]
    #[allow(non_snake_case)]
    fn FPDFPage_FormFieldZOrderAtPoint(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        page_x: f64,
        page_y: f64,
    ) -> c_int;

    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightColor(
        &self,
        handle: FPDF_FORMHANDLE,
        field_type: c_int,
        color: FPDF_DWORD,
    );

    #[allow(non_snake_case)]
    fn FPDF_SetFormFieldHighlightAlpha(&self, handle: FPDF_FORMHANDLE, alpha: c_uchar);

    #[doc = " Function: FPDF_RemoveFormFieldHighlight\n       Remove the form field highlight color in the document.\n Parameters:\n       hHandle     -   Handle to the form fill module, as returned by\n                       FPDFDOC_InitFormFillEnvironment().\n Return Value:\n       None.\n Comments:\n       Please refresh the client window to remove the highlight immediately\n       if necessary."]
    #[allow(non_snake_case)]
    fn FPDF_RemoveFormFieldHighlight(&self, hHandle: FPDF_FORMHANDLE);

    #[doc = " Function: FPDF_FFLDraw\n       Render FormFields and popup window on a page to a device independent\n       bitmap.\n Parameters:\n       hHandle      -   Handle to the form fill module, as returned by\n                        FPDFDOC_InitFormFillEnvironment().\n       bitmap       -   Handle to the device independent bitmap (as the\n                        output buffer). Bitmap handles can be created by\n                        FPDFBitmap_Create().\n       page         -   Handle to the page, as returned by FPDF_LoadPage().\n       start_x      -   Left pixel position of the display area in the\n                        device coordinates.\n       start_y      -   Top pixel position of the display area in the device\n                        coordinates.\n       size_x       -   Horizontal size (in pixels) for displaying the page.\n       size_y       -   Vertical size (in pixels) for displaying the page.\n       rotate       -   Page orientation: 0 (normal), 1 (rotated 90 degrees\n                        clockwise), 2 (rotated 180 degrees), 3 (rotated 90\n                        degrees counter-clockwise).\n       flags        -   0 for normal display, or combination of flags\n                        defined above.\n Return Value:\n       None.\n Comments:\n       This function is designed to render annotations that are\n       user-interactive, which are widget annotations (for FormFields) and\n       popup annotations.\n       With the FPDF_ANNOT flag, this function will render a popup annotation\n       when users mouse-hover on a non-widget annotation. Regardless of\n       FPDF_ANNOT flag, this function will always render widget annotations\n       for FormFields.\n       In order to implement the FormFill functions, implementation should\n       call this function after rendering functions, such as\n       FPDF_RenderPageBitmap() or FPDF_RenderPageBitmap_Start(), have\n       finished rendering the page contents."]
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDF_FFLDraw(
        &self,
        handle: FPDF_FORMHANDLE,
        bitmap: FPDF_BITMAP,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    );

    #[cfg(feature = "pdfium_use_skia")]
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    // TODO: AJRC - 24-Aug-24 no doc comment included in C headers, reuse FPDF_FFLDraw() doc comment
    fn FPDF_FFLDrawSkia(
        &self,
        hHandle: FPDF_FORMHANDLE,
        canvas: FPDF_SKIA_CANVAS,
        page: FPDF_PAGE,
        start_x: c_int,
        start_y: c_int,
        size_x: c_int,
        size_y: c_int,
        rotate: c_int,
        flags: c_int,
    );

    #[doc = " Experimental API\n Function: FPDF_GetFormType\n           Returns the type of form contained in the PDF document.\n Parameters:\n           document - Handle to document.\n Return Value:\n           Integer value representing one of the FORMTYPE_ values.\n Comments:\n           If |document| is NULL, then the return value is FORMTYPE_NONE."]
    #[allow(non_snake_case)]
    fn FPDF_GetFormType(&self, document: FPDF_DOCUMENT) -> c_int;

    #[doc = " Experimental API\n Function: FORM_SetIndexSelected\n           Selects/deselects the value at the given |index| of the focused\n           annotation.\n Parameters:\n           hHandle     -   Handle to the form fill module. Returned by\n                           FPDFDOC_InitFormFillEnvironment.\n           page        -   Handle to the page. Returned by FPDF_LoadPage\n           index       -   0-based index of value to be set as\n                           selected/unselected\n           selected    -   true to select, false to deselect\n Return Value:\n           TRUE if the operation succeeded.\n           FALSE if the operation failed or widget is not a supported type.\n Comments:\n           Intended for use with listbox/combobox widget types. Comboboxes\n           have at most a single value selected at a time which cannot be\n           deselected. Deselect on a combobox is a no-op that returns false.\n           Default implementation is a no-op that will return false for\n           other types.\n           Not currently supported for XFA forms - will return false."]
    #[allow(non_snake_case)]
    fn FORM_SetIndexSelected(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        index: c_int,
        selected: FPDF_BOOL,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API\n Function: FORM_IsIndexSelected\n           Returns whether or not the value at |index| of the focused\n           annotation is currently selected.\n Parameters:\n           hHandle     -   Handle to the form fill module. Returned by\n                           FPDFDOC_InitFormFillEnvironment.\n           page        -   Handle to the page. Returned by FPDF_LoadPage\n           index       -   0-based Index of value to check\n Return Value:\n           TRUE if value at |index| is currently selected.\n           FALSE if value at |index| is not selected or widget is not a\n           supported type.\n Comments:\n           Intended for use with listbox/combobox widget types. Default\n           implementation is a no-op that will return false for other types.\n           Not currently supported for XFA forms - will return false."]
    #[allow(non_snake_case)]
    fn FORM_IsIndexSelected(
        &self,
        hHandle: FPDF_FORMHANDLE,
        page: FPDF_PAGE,
        index: c_int,
    ) -> FPDF_BOOL;

    #[doc = " Function: FPDF_LoadXFA\n          If the document consists of XFA fields, call this method to\n          attempt to load XFA fields.\n Parameters:\n          document     -   Handle to document from FPDF_LoadDocument().\n Return Value:\n          TRUE upon success, otherwise FALSE. If XFA support is not built\n          into PDFium, performs no action and always returns FALSE."]
    #[allow(non_snake_case)]
    fn FPDF_LoadXFA(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Get the number of JavaScript actions in |document|.\n\n   document - handle to a document.\n\n Returns the number of JavaScript actions in |document| or -1 on error."]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptActionCount(&self, document: FPDF_DOCUMENT) -> c_int;

    #[doc = " Experimental API.\n Get the JavaScript action at |index| in |document|.\n\n   document - handle to a document.\n   index    - the index of the requested JavaScript action.\n\n Returns the handle to the JavaScript action, or NULL on failure.\n Caller owns the returned handle and must close it with\n FPDFDoc_CloseJavaScriptAction()."]
    #[allow(non_snake_case)]
    fn FPDFDoc_GetJavaScriptAction(
        &self,
        document: FPDF_DOCUMENT,
        index: c_int,
    ) -> FPDF_JAVASCRIPT_ACTION;

    #[doc = "   javascript - Handle to a JavaScript action."]
    #[allow(non_snake_case)]
    fn FPDFDoc_CloseJavaScriptAction(&self, javascript: FPDF_JAVASCRIPT_ACTION);

    #[doc = " Experimental API.\n Get the name from the |javascript| handle. |buffer| is only modified if\n |buflen| is longer than the length of the name. On errors, |buffer| is\n unmodified and the returned length is 0.\n\n   javascript - handle to an JavaScript action.\n   buffer     - buffer for holding the name, encoded in UTF-16LE.\n   buflen     - length of the buffer in bytes.\n\n Returns the length of the JavaScript action name in bytes."]
    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetName(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    #[doc = " Experimental API.\n Get the script from the |javascript| handle. |buffer| is only modified if\n |buflen| is longer than the length of the script. On errors, |buffer| is\n unmodified and the returned length is 0.\n\n   javascript - handle to an JavaScript action.\n   buffer     - buffer for holding the name, encoded in UTF-16LE.\n   buflen     - length of the buffer in bytes.\n\n Returns the length of the JavaScript action name in bytes."]
    #[allow(non_snake_case)]
    fn FPDFJavaScriptAction_GetScript(
        &self,
        javascript: FPDF_JAVASCRIPT_ACTION,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    #[doc = " Function: FPDF_GetDefaultTTFMap\n    Returns a pointer to the default character set to TT Font name map. The\n    map is an array of FPDF_CharsetFontMap structs, with its end indicated\n    by a { -1, NULL } entry.\n Parameters:\n     None.\n Return Value:\n     Pointer to the Charset Font Map.\n Note:\n     Once FPDF_GetDefaultTTFMapCount() and FPDF_GetDefaultTTFMapEntry() are no\n     longer experimental, this API will be marked as deprecated.\n     See https://crbug.com/348468114"]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMap(&self) -> *const FPDF_CharsetFontMap;

    #[cfg(any(
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[doc = " Experimental API.\n\n Function: FPDF_GetDefaultTTFMapCount\n    Returns the number of entries in the default character set to TT Font name\n    map.\n Parameters:\n    None.\n Return Value:\n    The number of entries in the map."]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapCount(&self) -> usize;

    #[cfg(any(
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[doc = " Experimental API.\n\n Function: FPDF_GetDefaultTTFMapEntry\n    Returns an entry in the default character set to TT Font name map.\n Parameters:\n    index    -   The index to the entry in the map to retrieve.\n Return Value:\n     A pointer to the entry, if it is in the map, or NULL if the index is out\n     of bounds."]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultTTFMapEntry(&self, index: usize) -> *const FPDF_CharsetFontMap;

    #[doc = " Function: FPDF_AddInstalledFont\n          Add a system font to the list in PDFium.\n Comments:\n          This function is only called during the system font list building\n          process.\n Parameters:\n          mapper          -   Opaque pointer to Foxit font mapper\n          face            -   The font face name\n          charset         -   Font character set. See above defined constants.\n Return Value:\n          None."]
    #[allow(non_snake_case)]
    fn FPDF_AddInstalledFont(&self, mapper: *mut c_void, face: *const c_char, charset: c_int);

    #[doc = " Function: FPDF_SetSystemFontInfo\n          Set the system font info interface into PDFium\n Parameters:\n          pFontInfo       -   Pointer to a FPDF_SYSFONTINFO structure\n Return Value:\n          None\n Comments:\n          Platform support implementation should implement required methods of\n          FFDF_SYSFONTINFO interface, then call this function during PDFium\n          initialization process.\n\n          Call this with NULL to tell PDFium to stop using a previously set\n          |FPDF_SYSFONTINFO|."]
    #[allow(non_snake_case)]
    fn FPDF_SetSystemFontInfo(&self, pFontInfo: *mut FPDF_SYSFONTINFO);

    #[doc = " Function: FPDF_GetDefaultSystemFontInfo\n          Get default system font info interface for current platform\n Parameters:\n          None\n Return Value:\n          Pointer to a FPDF_SYSFONTINFO structure describing the default\n          interface, or NULL if the platform doesn't have a default interface.\n          Application should call FPDF_FreeDefaultSystemFontInfo to free the\n          returned pointer.\n Comments:\n          For some platforms, PDFium implements a default version of system\n          font info interface. The default implementation can be passed to\n          FPDF_SetSystemFontInfo()."]
    #[allow(non_snake_case)]
    fn FPDF_GetDefaultSystemFontInfo(&self) -> *mut FPDF_SYSFONTINFO;

    #[doc = " Function: FPDF_FreeDefaultSystemFontInfo\n           Free a default system font info interface\n Parameters:\n           pFontInfo       -   Pointer to a FPDF_SYSFONTINFO structure\n Return Value:\n           None\n Comments:\n           This function should be called on the output from\n           FPDF_GetDefaultSystemFontInfo() once it is no longer needed."]
    #[allow(non_snake_case)]
    fn FPDF_FreeDefaultSystemFontInfo(&self, pFontInfo: *mut FPDF_SYSFONTINFO);

    /// Gets the first child of `bookmark`, or the first top-level bookmark item.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `bookmark` - handle to the current bookmark. Pass `NULL` for the first top
    ///                level item.
    ///
    /// Returns a handle to the first child of `bookmark` or the first top-level
    /// bookmark item. `NULL` if no child or top-level bookmark found.
    /// Note that another name for the bookmarks is the document outline, as
    /// described in ISO 32000-1:2008, section 12.3.3.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetFirstChild(
        &self,
        document: FPDF_DOCUMENT,
        bookmark: FPDF_BOOKMARK,
    ) -> FPDF_BOOKMARK;

    /// Gets the next sibling of `bookmark`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `bookmark` - handle to the current bookmark.
    ///
    /// Returns a handle to the next sibling of `bookmark`, or `NULL` if this is the
    /// last bookmark at this level.
    ///
    /// Note that the caller is responsible for handling circular bookmark
    /// references, as may arise from malformed documents.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetNextSibling(
        &self,
        document: FPDF_DOCUMENT,
        bookmark: FPDF_BOOKMARK,
    ) -> FPDF_BOOKMARK;

    /// Gets the title of `bookmark`.
    ///
    ///   `bookmark` - handle to the bookmark.
    ///
    ///   `buffer`   - buffer for the title. May be `NULL`.
    ///
    ///   `buflen`   - the length of the buffer in bytes. May be 0.
    ///
    /// Returns the number of bytes in the title, including the terminating `NUL`
    /// character. The number of bytes is returned regardless of the `buffer` and
    /// `buflen` parameters.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-16LE encoding. The
    /// string is terminated by a UTF16 `NUL` character. If `buflen` is less than the
    /// required length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetTitle(
        &self,
        bookmark: FPDF_BOOKMARK,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the number of children of `bookmark`.
    ///
    ///   `bookmark` - handle to the bookmark.
    ///
    /// Returns a signed integer that represents the number of sub-items the given
    /// bookmark has. If the value is positive, child items shall be shown by default
    /// (open state). If the value is negative, child items shall be hidden by
    /// default (closed state). Please refer to PDF 32000-1:2008, Table 153.
    /// Returns 0 if the bookmark has no children or is invalid.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetCount(&self, bookmark: FPDF_BOOKMARK) -> c_int;

    /// Finds the bookmark with `title` in `document`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `title`    - the UTF-16LE encoded Unicode title for which to search.
    ///
    /// Returns the handle to the bookmark, or `NULL` if `title` can't be found.
    ///
    /// `FPDFBookmark_Find()` will always return the first bookmark found even if
    /// multiple bookmarks have the same `title`.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFBookmark_Find_str].
    #[allow(non_snake_case)]
    fn FPDFBookmark_Find(&self, document: FPDF_DOCUMENT, title: FPDF_WIDESTRING) -> FPDF_BOOKMARK;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFBookmark_Find].
    ///
    /// Finds the bookmark with `title` in `document`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `title`    - the title for which to search.
    ///
    /// Returns the handle to the bookmark, or `NULL` if `title` can't be found.
    ///
    /// `FPDFBookmark_Find_str()` will always return the first bookmark found even if
    /// multiple bookmarks have the same `title`.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFBookmark_Find_str(&self, document: FPDF_DOCUMENT, title: &str) -> FPDF_BOOKMARK {
        self.FPDFBookmark_Find(
            document,
            get_pdfium_utf16le_bytes_from_str(title).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Gets the destination associated with `bookmark`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `bookmark` - handle to the bookmark.
    ///
    /// Returns the handle to the destination data, or `NULL` if no destination is
    /// associated with `bookmark`.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetDest(&self, document: FPDF_DOCUMENT, bookmark: FPDF_BOOKMARK) -> FPDF_DEST;

    /// Gets the action associated with `bookmark`.
    ///
    ///   `bookmark` - handle to the bookmark.
    ///
    /// Returns the handle to the action data, or `NULL` if no action is associated
    /// with `bookmark`.
    ///
    /// If this function returns a valid handle, it is valid as long as `bookmark` is
    /// valid.
    ///
    /// If this function returns `NULL`, `FPDFBookmark_GetDest()` should be called to get
    /// the `bookmark` destination data.
    #[allow(non_snake_case)]
    fn FPDFBookmark_GetAction(&self, bookmark: FPDF_BOOKMARK) -> FPDF_ACTION;

    /// Gets the type of `action`.
    ///
    ///   `action` - handle to the action.
    ///
    /// Returns one of:
    ///   - `PDFACTION_UNSUPPORTED`
    ///   - `PDFACTION_GOTO`
    ///   - `PDFACTION_REMOTEGOTO`
    ///   - `PDFACTION_URI`
    ///   - `PDFACTION_LAUNCH`
    #[allow(non_snake_case)]
    fn FPDFAction_GetType(&self, action: FPDF_ACTION) -> c_ulong;

    /// Gets the destination of `action`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `action`   - handle to the action. `action` must be a `PDFACTION_GOTO` or
    ///                `PDFACTION_REMOTEGOTO`.
    ///
    /// Returns a handle to the destination data, or `NULL` on error, typically
    /// because the arguments were bad or the action was of the wrong type.
    ///
    /// In the case of `PDFACTION_REMOTEGOTO`, you must first call
    /// `FPDFAction_GetFilePath()`, then load the document at that path, then pass
    /// the document handle from that document as `document` to `FPDFAction_GetDest()`.
    #[allow(non_snake_case)]
    fn FPDFAction_GetDest(&self, document: FPDF_DOCUMENT, action: FPDF_ACTION) -> FPDF_DEST;

    /// Gets the file path of `action`.
    ///
    ///   `action` - handle to the action. `action` must be a `PDFACTION_LAUNCH` or
    ///              `PDFACTION_REMOTEGOTO`.
    ///
    ///   `buffer` - a buffer for output the path string. May be `NULL`.
    ///
    ///   `buflen` - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the file path, including the trailing `NUL`
    /// character, or 0 on error, typically because the arguments were bad or the
    /// action was of the wrong type.
    ///
    /// Regardless of the platform, the `buffer` is always in UTF-8 encoding.
    /// If `buflen` is less than the returned length, or `buffer` is `NULL`, `buffer`
    /// will not be modified.
    #[allow(non_snake_case)]
    fn FPDFAction_GetFilePath(
        &self,
        action: FPDF_ACTION,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the URI path of `action`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `action`   - handle to the action. Must be a `PDFACTION_URI`.
    ///
    ///   `buffer`   - a buffer for the path string. May be `NULL`.
    ///
    ///   `buflen`   - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the URI path, including the trailing `NUL`
    /// character, or 0 on error, typically because the arguments were bad or the
    /// action was of the wrong type.
    ///
    /// The `buffer` may contain badly encoded data. The caller should validate the
    /// output, i.e. check to see if it is UTF-8.
    ///
    /// If `buflen` is less than the returned length, or `buffer` is `NULL`, buffer`
    /// will not be modified.
    ///
    /// Historically, the documentation for this API claimed `buffer` is always
    /// encoded in 7-bit ASCII, but did not actually enforce it.
    /// <https://pdfium.googlesource.com/pdfium.git/+/d609e84cee2e14a18333247485af91df48a40592>
    /// added that enforcement, but that did not work well for real world PDFs that
    /// used UTF-8. As of this writing, this API reverted back to its original
    /// behavior prior to commit d609e84cee.
    #[allow(non_snake_case)]
    fn FPDFAction_GetURIPath(
        &self,
        document: FPDF_DOCUMENT,
        action: FPDF_ACTION,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the page index of `dest`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `dest`     - handle to the destination.
    ///
    /// Returns the 0-based page index containing `dest`. Returns -1 on error.
    #[allow(non_snake_case)]
    fn FPDFDest_GetDestPageIndex(&self, document: FPDF_DOCUMENT, dest: FPDF_DEST) -> c_int;

    /// Gets the view (fit type) specified by `dest`.
    ///
    ///   `dest`         - handle to the destination.
    ///
    ///   `pNumParams`   - receives the number of view parameters, which is at most 4.
    ///
    ///   `pParams`      - buffer to write the view parameters. Must be at least 4
    ///                    `FS_FLOAT`s long.
    ///
    /// Returns one of the `PDFDEST_VIEW_*` constants, or `PDFDEST_VIEW_UNKNOWN_MODE` if
    /// `dest` does not specify a view.
    #[allow(non_snake_case)]
    fn FPDFDest_GetView(
        &self,
        dest: FPDF_DEST,
        pNumParams: *mut c_ulong,
        pParams: *mut FS_FLOAT,
    ) -> c_ulong;

    /// Gets the (`x`, `y`, `zoom`) location of `dest` in the destination page, if the
    /// destination is in `page /XYZ x y zoom` syntax.
    ///
    ///   `dest`       - handle to the destination.
    ///
    ///   `hasXVal`    - out parameter; true if the `x` value is not null
    ///
    ///   `hasYVal`    - out parameter; true if the `y` value is not null
    ///
    ///   `hasZoomVal` - out parameter; true if the `zoom` value is not null
    ///
    ///   `x`          - out parameter; the `x` coordinate, in page coordinates.
    ///
    ///   `y`          - out parameter; the `y` coordinate, in page coordinates.
    ///
    ///   `zoom`       - out parameter; the `zoom` value.
    ///
    /// Returns `true` on successfully reading the `/XYZ` value.
    ///
    /// Note the `x`, `y`, `zoom` values are only set if the corresponding `hasXVal`,
    /// `hasYVal`, or `hasZoomVal` flags are true.
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFDest_GetLocationInPage(
        &self,
        dest: FPDF_DEST,
        hasXVal: *mut FPDF_BOOL,
        hasYVal: *mut FPDF_BOOL,
        hasZoomVal: *mut FPDF_BOOL,
        x: *mut FS_FLOAT,
        y: *mut FS_FLOAT,
        zoom: *mut FS_FLOAT,
    ) -> FPDF_BOOL;

    /// Finds a link at point (`x`, `y`) on `page`.
    ///
    ///   `page` - handle to the document page.
    ///
    ///   `x`    - the `x` coordinate, in the page coordinate system.
    ///
    ///   `y`    - the `y` coordinate, in the page coordinate system.
    ///
    /// Returns a handle to the link, or `NULL` if no link found at the given point.
    ///
    /// You can convert coordinates from screen coordinates to page coordinates using
    /// `FPDF_DeviceToPage()`.
    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> FPDF_LINK;

    /// Finds the Z-order of link at point (`x`, `y`) on `page`.
    ///
    ///   `page` - handle to the document page.
    ///
    ///   `x`    - the `x` coordinate, in the page coordinate system.
    ///
    ///   `y`    - the `y` coordinate, in the page coordinate system.
    ///
    /// Returns the Z-order of the link, or -1 if no link found at the given point.
    /// Larger Z-order numbers are closer to the front.
    ///
    /// You can convert coordinates from screen coordinates to page coordinates using
    /// `FPDF_DeviceToPage()`.
    #[allow(non_snake_case)]
    fn FPDFLink_GetLinkZOrderAtPoint(&self, page: FPDF_PAGE, x: c_double, y: c_double) -> c_int;

    /// Gets destination info for `link`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `link`     - handle to the link.
    ///
    /// Returns a handle to the destination, or `NULL` if there is no destination
    /// associated with the link. In this case, you should call `FPDFLink_GetAction()`
    /// to retrieve the action associated with `link`.
    #[allow(non_snake_case)]
    fn FPDFLink_GetDest(&self, document: FPDF_DOCUMENT, link: FPDF_LINK) -> FPDF_DEST;

    /// Gets action info for `link`.
    ///
    ///   `link` - handle to the link.
    ///
    /// Returns a handle to the action associated to `link`, or `NULL` if no action.
    /// If this function returns a valid handle, it is valid as long as `link` is
    /// valid.
    #[allow(non_snake_case)]
    fn FPDFLink_GetAction(&self, link: FPDF_LINK) -> FPDF_ACTION;

    /// Enumerates all the link annotations in `page`.
    ///
    ///   `page`       - handle to the page.
    ///
    ///   `start_pos`  - the start position, should initially be 0 and is updated with
    ///                  the next start position on return.
    ///
    ///   `link_annot` - the link handle for `startPos`.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFLink_Enumerate(
        &self,
        page: FPDF_PAGE,
        start_pos: *mut c_int,
        link_annot: *mut FPDF_LINK,
    ) -> FPDF_BOOL;

    /// Gets `FPDF_ANNOTATION` object for `link_annot`.
    ///
    ///   `page`       - handle to the page in which `FPDF_LINK` object is present.
    ///
    ///   `link_annot` - handle to link annotation.
    ///
    /// Returns `FPDF_ANNOTATION` from the `FPDF_LINK` or `NULL` on failure,
    /// if the input link annot or page is `NULL`.
    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnot(&self, page: FPDF_PAGE, link_annot: FPDF_LINK) -> FPDF_ANNOTATION;

    /// Gets the rectangle for `link_annot`.
    ///
    ///   `link_annot` - handle to the link annotation.
    ///
    ///   `rect`       - the annotation rectangle.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFLink_GetAnnotRect(&self, link_annot: FPDF_LINK, rect: *mut FS_RECTF) -> FPDF_BOOL;

    /// Gets the count of quadrilateral points to the `link_annot`.
    ///
    ///   `link_annot` - handle to the link annotation.
    ///
    /// Returns the count of quadrilateral points.
    #[allow(non_snake_case)]
    fn FPDFLink_CountQuadPoints(&self, link_annot: FPDF_LINK) -> c_int;

    /// Gets the quadrilateral points for the specified `quad_index` in `link_annot`.
    ///
    ///   `link_annot`  - handle to the link annotation.
    ///
    ///   `quad_index`  - the specified quad point index.
    ///
    ///   `quad_points` - receives the quadrilateral points.
    ///
    /// Returns `true` on success.
    #[allow(non_snake_case)]
    fn FPDFLink_GetQuadPoints(
        &self,
        link_annot: FPDF_LINK,
        quad_index: c_int,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL;

    /// Gets an additional-action from `page`.
    ///
    ///   `page`      - handle to the page, as returned by `FPDF_LoadPage()`.
    ///
    ///   `aa_type`   - the type of the page object's additional-action, defined
    ///                 in `public/fpdf_formfill.h`
    ///
    ///   Returns the handle to the action data, or `NULL` if there is no
    ///   additional-action of type `aa_type`.
    ///
    ///   If this function returns a valid handle, it is valid as long as `page` is
    ///   valid.
    #[allow(non_snake_case)]
    fn FPDF_GetPageAAction(&self, page: FPDF_PAGE, aa_type: c_int) -> FPDF_ACTION;

    /// Gets the file identifier defined in the trailer of `document`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `id_type`  - the file identifier type to retrieve.
    ///
    ///   `buffer`   - a buffer for the file identifier. May be `NULL`.
    ///
    ///   `buflen`   - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the file identifier, including the `NUL`
    /// terminator.
    ///
    /// The `buffer` is always a byte string. The `buffer` is followed by a `NUL`
    /// terminator.  If `buflen` is less than the returned length, or `buffer` is
    /// `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_GetFileIdentifier(
        &self,
        document: FPDF_DOCUMENT,
        id_type: FPDF_FILEIDTYPE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets meta-data `tag` content from `document`.
    ///
    ///   `document` - handle to the document.
    ///
    ///   `tag`      - the tag to retrieve. The tag can be one of:
    ///                Title, Author, Subject, Keywords, Creator, Producer,
    ///                CreationDate, or ModDate.
    ///                For detailed explanations of these tags and their respective
    ///                values, please refer to PDF Reference 1.6, section 10.2.1,
    ///                "Document Information Dictionary".
    ///
    ///   `buffer`   - a buffer for the tag. May be `NULL`.
    ///
    ///   `buflen`   - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the tag, including trailing zeros.
    ///
    /// The |buffer| is always encoded in UTF-16LE. The `buffer` is followed by two
    /// bytes of zeros indicating the end of the string.  If `buflen` is less than
    /// the returned length, or `buffer` is `NULL`, `buffer` will not be modified.
    ///
    /// For linearized files, `FPDFAvail_IsFormAvail()` must be called before this, and
    /// it must have returned `PDF_FORM_AVAIL` or `PDF_FORM_NOTEXIST`. Before that, there
    /// is no guarantee the metadata has been loaded.
    #[allow(non_snake_case)]
    fn FPDF_GetMetaText(
        &self,
        document: FPDF_DOCUMENT,
        tag: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Gets the page label for `page_index` from `document`.
    ///
    ///   `document`    - handle to the document.
    ///
    ///   `page_index`  - the 0-based index of the page.
    ///
    ///   `buffer`      - a buffer for the page label. May be `NULL`.
    ///
    ///   `buflen`      - the length of the buffer, in bytes. May be 0.
    ///
    /// Returns the number of bytes in the page label, including trailing zeros.
    ///
    /// The `buffer` is always encoded in UTF-16LE. The `buffer` is followed by two
    /// bytes of zeros indicating the end of the string.  If `buflen` is less than
    /// the returned length, or `buffer` is `NULL`, `buffer` will not be modified.
    #[allow(non_snake_case)]
    fn FPDF_GetPageLabel(
        &self,
        document: FPDF_DOCUMENT,
        page_index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    #[cfg(feature = "pdfium_enable_xfa")]
    #[doc = " Experimental API.\n Function: FPDF_GetXFAPacketCount\n          Get the number of valid packets in the XFA entry.\n Parameters:\n          document - Handle to the document.\n Return value:\n          The number of valid packets, or -1 on error."]
    #[allow(non_snake_case)]
    fn FPDF_GetXFAPacketCount(&self, document: FPDF_DOCUMENT) -> c_int;

    #[cfg(feature = "pdfium_enable_xfa")]
    #[doc = " Experimental API.\n Function: FPDF_GetXFAPacketName\n          Get the name of a packet in the XFA array.\n Parameters:\n          document - Handle to the document.\n          index    - Index number of the packet. 0 for the first packet.\n          buffer   - Buffer for holding the name of the XFA packet.\n          buflen   - Length of |buffer| in bytes.\n Return value:\n          The length of the packet name in bytes, or 0 on error.\n\n |document| must be valid and |index| must be in the range [0, N), where N is\n the value returned by FPDF_GetXFAPacketCount().\n |buffer| is only modified if it is non-NULL and |buflen| is greater than or\n equal to the length of the packet name. The packet name includes a\n terminating NUL character. |buffer| is unmodified on error."]
    #[allow(non_snake_case)]
    fn FPDF_GetXFAPacketName(
        &self,
        document: FPDF_DOCUMENT,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    #[cfg(feature = "pdfium_enable_xfa")]
    #[doc = " Experimental API.\n Function: FPDF_GetXFAPacketContent\n          Get the content of a packet in the XFA array.\n Parameters:\n          document   - Handle to the document.\n          index      - Index number of the packet. 0 for the first packet.\n          buffer     - Buffer for holding the content of the XFA packet.\n          buflen     - Length of |buffer| in bytes.\n          out_buflen - Pointer to the variable that will receive the minimum\n                       buffer size needed to contain the content of the XFA\n                       packet.\n Return value:\n          Whether the operation succeeded or not.\n\n |document| must be valid and |index| must be in the range [0, N), where N is\n the value returned by FPDF_GetXFAPacketCount(). |out_buflen| must not be\n NULL. When the aforementioned arguments are valid, the operation succeeds,\n and |out_buflen| receives the content size. |buffer| is only modified if\n |buffer| is non-null and long enough to contain the content. Callers must\n check both the return value and the input |buflen| is no less than the\n returned |out_buflen| before using the data in |buffer|."]
    #[allow(non_snake_case)]
    fn FPDF_GetXFAPacketContent(
        &self,
        document: FPDF_DOCUMENT,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[cfg(feature = "pdfium_enable_v8")]
    #[doc = " Function: FPDF_GetRecommendedV8Flags\n          Returns a space-separated string of command line flags that are\n          recommended to be passed into V8 via V8::SetFlagsFromString()\n          prior to initializing the PDFium library.\n Parameters:\n          None.\n Return value:\n          NUL-terminated string of the form \"--flag1 --flag2\".\n          The caller must not attempt to modify or free the result."]
    #[allow(non_snake_case)]
    fn FPDF_GetRecommendedV8Flags(&self) -> *const c_char;

    #[cfg(feature = "pdfium_enable_v8")]
    #[doc = " Experimental API.\n Function: FPDF_GetArrayBufferAllocatorSharedInstance()\n          Helper function for initializing V8 isolates that will\n          use PDFium's internal memory management.\n Parameters:\n          None.\n Return Value:\n          Pointer to a suitable v8::ArrayBuffer::Allocator, returned\n          as void for C compatibility.\n Notes:\n          Use is optional, but allows external creation of isolates\n          matching the ones PDFium will make when none is provided\n          via |FPDF_LIBRARY_CONFIG::m_pIsolate|.\n\n          Can only be called when the library is in an uninitialized or\n          destroyed state."]
    #[allow(non_snake_case)]
    fn FPDF_GetArrayBufferAllocatorSharedInstance(&self) -> *mut c_void;

    #[cfg(feature = "pdfium_enable_xfa")]
    #[doc = " Function: FPDF_BStr_Init\n          Helper function to initialize a FPDF_BSTR."]
    #[allow(non_snake_case)]
    fn FPDF_BStr_Init(&self, bstr: *mut FPDF_BSTR) -> FPDF_RESULT;

    #[cfg(feature = "pdfium_enable_xfa")]
    #[doc = " Function: FPDF_BStr_Set\n          Helper function to copy string data into the FPDF_BSTR."]
    #[allow(non_snake_case)]
    fn FPDF_BStr_Set(
        &self,
        bstr: *mut FPDF_BSTR,
        cstr: *const c_char,
        length: c_int,
    ) -> FPDF_RESULT;

    #[cfg(feature = "pdfium_enable_xfa")]
    #[doc = " Function: FPDF_BStr_Clear\n          Helper function to clear a FPDF_BSTR."]
    #[allow(non_snake_case)]
    fn FPDF_BStr_Clear(&self, bstr: *mut FPDF_BSTR) -> FPDF_RESULT;

    #[doc = " Function: FPDFText_LoadPage\n          Prepare information about all characters in a page.\n Parameters:\n          page    -   Handle to the page. Returned by FPDF_LoadPage function\n                      (in FPDFVIEW module).\n Return value:\n          A handle to the text page information structure.\n          NULL if something goes wrong.\n Comments:\n          Application must call FPDFText_ClosePage to release the text page\n          information.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_LoadPage(&self, page: FPDF_PAGE) -> FPDF_TEXTPAGE;

    #[doc = " Function: FPDFText_ClosePage\n          Release all resources allocated for a text page information\n          structure.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n Return Value:\n          None.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_ClosePage(&self, text_page: FPDF_TEXTPAGE);

    #[doc = " Function: FPDFText_CountChars\n          Get number of characters in a page.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n Return value:\n          Number of characters in the page. Return -1 for error.\n          Generated characters, like additional space characters, new line\n          characters, are also counted.\n Comments:\n          Characters in a page form a \"stream\", inside the stream, each\n          character has an index.\n          We will use the index parameters in many of FPDFTEXT functions. The\n          first character in the page\n          has an index value of zero.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_CountChars(&self, text_page: FPDF_TEXTPAGE) -> c_int;

    #[doc = " Function: FPDFText_GetUnicode\n          Get Unicode of a character in a page.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n Return value:\n          The Unicode of the particular character.\n          If a character is not encoded in Unicode and Foxit engine can't\n          convert to Unicode,\n          the return value will be zero.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetUnicode(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_uint;

    #[cfg(any(
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[doc = " Experimental API.\n Function: FPDFText_GetTextObject\n          Get the FPDF_PAGEOBJECT associated with a given character.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n Return value:\n          The associated text object for the character at |index|, or NULL on\n          error. The returned text object, if non-null, is of type\n          |FPDF_PAGEOBJ_TEXT|. The caller does not own the returned object.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetTextObject(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> FPDF_PAGEOBJECT;

    #[doc = " Experimental API.\n Function: FPDFText_IsGenerated\n          Get if a character in a page is generated by PDFium.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n Return value:\n          1 if the character is generated by PDFium.\n          0 if the character is not generated by PDFium.\n          -1 if there was an error.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_IsGenerated(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int;

    #[cfg(any(
        feature = "pdfium_6015",
        feature = "pdfium_6043",
        feature = "pdfium_6084",
        feature = "pdfium_6110",
        feature = "pdfium_6124",
        feature = "pdfium_6164",
        feature = "pdfium_6259",
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406",
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[doc = " Experimental API.\n Function: FPDFText_IsHyphen\n          Get if a character in a page is a hyphen.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n Return value:\n          1 if the character is a hyphen.\n          0 if the character is not a hyphen.\n          -1 if there was an error.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_IsHyphen(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int;

    #[doc = " Experimental API.\n Function: FPDFText_HasUnicodeMapError\n          Get if a character in a page has an invalid unicode mapping.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n Return value:\n          1 if the character has an invalid unicode mapping.\n          0 if the character has no known unicode mapping issues.\n          -1 if there was an error.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_HasUnicodeMapError(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int;

    #[doc = " Function: FPDFText_GetFontSize\n          Get the font size of a particular character.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n Return value:\n          The font size of the particular character, measured in points (about\n          1/72 inch). This is the typographic size of the font (so called\n          \"em size\").\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetFontSize(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_double;

    #[doc = " Experimental API.\n Function: FPDFText_GetFontInfo\n          Get the font name and flags of a particular character.\n Parameters:\n          text_page - Handle to a text page information structure.\n                      Returned by FPDFText_LoadPage function.\n          index     - Zero-based index of the character.\n          buffer    - A buffer receiving the font name.\n          buflen    - The length of |buffer| in bytes.\n          flags     - Optional pointer to an int receiving the font flags.\n                      These flags should be interpreted per PDF spec 1.7\n                      Section 5.7.1 Font Descriptor Flags.\n Return value:\n          On success, return the length of the font name, including the\n          trailing NUL character, in bytes. If this length is less than or\n          equal to |length|, |buffer| is set to the font name, |flags| is\n          set to the font flags. |buffer| is in UTF-8 encoding. Return 0 on\n          failure.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetFontInfo(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
        flags: *mut c_int,
    ) -> c_ulong;

    #[doc = " Experimental API.\n Function: FPDFText_GetFontWeight\n          Get the font weight of a particular character.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n Return value:\n          On success, return the font weight of the particular character. If\n          |text_page| is invalid, if |index| is out of bounds, or if the\n          character's text object is undefined, return -1.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetFontWeight(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_int;

    #[cfg(any(
        feature = "pdfium_6569",
        feature = "pdfium_6555",
        feature = "pdfium_6490",
        feature = "pdfium_6406",
        feature = "pdfium_6337",
        feature = "pdfium_6295",
        feature = "pdfium_6259",
        feature = "pdfium_6164",
        feature = "pdfium_6124",
        feature = "pdfium_6110",
        feature = "pdfium_6084",
        feature = "pdfium_6043",
        feature = "pdfium_6015",
        feature = "pdfium_5961"
    ))]
    #[doc = " Experimental API.\n Function: FPDFText_GetTextRenderMode\n          Get text rendering mode of character.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n Return Value:\n          On success, return the render mode value. A valid value is of type\n          FPDF_TEXT_RENDERMODE. If |text_page| is invalid, if |index| is out\n          of bounds, or if the text object is undefined, then return\n          FPDF_TEXTRENDERMODE_UNKNOWN.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetTextRenderMode(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
    ) -> FPDF_TEXT_RENDERMODE;

    #[doc = " Experimental API.\n Function: FPDFText_GetFillColor\n          Get the fill color of a particular character.\n Parameters:\n          text_page      -   Handle to a text page information structure.\n                             Returned by FPDFText_LoadPage function.\n          index          -   Zero-based index of the character.\n          R              -   Pointer to an unsigned int number receiving the\n                             red value of the fill color.\n          G              -   Pointer to an unsigned int number receiving the\n                             green value of the fill color.\n          B              -   Pointer to an unsigned int number receiving the\n                             blue value of the fill color.\n          A              -   Pointer to an unsigned int number receiving the\n                             alpha value of the fill color.\n Return value:\n          Whether the call succeeded. If false, |R|, |G|, |B| and |A| are\n          unchanged.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetFillColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FPDFText_GetStrokeColor\n          Get the stroke color of a particular character.\n Parameters:\n          text_page      -   Handle to a text page information structure.\n                             Returned by FPDFText_LoadPage function.\n          index          -   Zero-based index of the character.\n          R              -   Pointer to an unsigned int number receiving the\n                             red value of the stroke color.\n          G              -   Pointer to an unsigned int number receiving the\n                             green value of the stroke color.\n          B              -   Pointer to an unsigned int number receiving the\n                             blue value of the stroke color.\n          A              -   Pointer to an unsigned int number receiving the\n                             alpha value of the stroke color.\n Return value:\n          Whether the call succeeded. If false, |R|, |G|, |B| and |A| are\n          unchanged.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetStrokeColor(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FPDFText_GetCharAngle\n          Get character rotation angle.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n Return Value:\n          On success, return the angle value in radian. Value will always be\n          greater or equal to 0. If |text_page| is invalid, or if |index| is\n          out of bounds, then return -1.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharAngle(&self, text_page: FPDF_TEXTPAGE, index: c_int) -> c_float;

    #[doc = " Function: FPDFText_GetCharBox\n          Get bounding box of a particular character.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n          left        -   Pointer to a double number receiving left position\n                          of the character box.\n          right       -   Pointer to a double number receiving right position\n                          of the character box.\n          bottom      -   Pointer to a double number receiving bottom position\n                          of the character box.\n          top         -   Pointer to a double number receiving top position of\n                          the character box.\n Return Value:\n          On success, return TRUE and fill in |left|, |right|, |bottom|, and\n          |top|. If |text_page| is invalid, or if |index| is out of bounds,\n          then return FALSE, and the out parameters remain unmodified.\n Comments:\n          All positions are measured in PDF \"user space\".\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharBox(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        left: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
        top: *mut c_double,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FPDFText_GetLooseCharBox\n          Get a \"loose\" bounding box of a particular character, i.e., covering\n          the entire glyph bounds, without taking the actual glyph shape into\n          account.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n          rect        -   Pointer to a FS_RECTF receiving the character box.\n Return Value:\n          On success, return TRUE and fill in |rect|. If |text_page| is\n          invalid, or if |index| is out of bounds, then return FALSE, and the\n          |rect| out parameter remains unmodified.\n Comments:\n          All positions are measured in PDF \"user space\".\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetLooseCharBox(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        rect: *mut FS_RECTF,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FPDFText_GetMatrix\n          Get the effective transformation matrix for a particular character.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage().\n          index       -   Zero-based index of the character.\n          matrix      -   Pointer to a FS_MATRIX receiving the transformation\n                          matrix.\n Return Value:\n          On success, return TRUE and fill in |matrix|. If |text_page| is\n          invalid, or if |index| is out of bounds, or if |matrix| is NULL,\n          then return FALSE, and |matrix| remains unmodified.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetMatrix(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        matrix: *mut FS_MATRIX,
    ) -> FPDF_BOOL;

    #[doc = " Function: FPDFText_GetCharOrigin\n          Get origin of a particular character.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          index       -   Zero-based index of the character.\n          x           -   Pointer to a double number receiving x coordinate of\n                          the character origin.\n          y           -   Pointer to a double number receiving y coordinate of\n                          the character origin.\n Return Value:\n          Whether the call succeeded. If false, x and y are unchanged.\n Comments:\n          All positions are measured in PDF \"user space\".\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharOrigin(
        &self,
        text_page: FPDF_TEXTPAGE,
        index: c_int,
        x: *mut c_double,
        y: *mut c_double,
    ) -> FPDF_BOOL;

    #[doc = " Function: FPDFText_GetCharIndexAtPos\n          Get the index of a character at or nearby a certain position on the\n          page.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          x           -   X position in PDF \"user space\".\n          y           -   Y position in PDF \"user space\".\n          xTolerance  -   An x-axis tolerance value for character hit\n                          detection, in point units.\n          yTolerance  -   A y-axis tolerance value for character hit\n                          detection, in point units.\n Return Value:\n          The zero-based index of the character at, or nearby the point (x,y).\n          If there is no character at or nearby the point, return value will\n          be -1. If an error occurs, -3 will be returned.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetCharIndexAtPos(
        &self,
        text_page: FPDF_TEXTPAGE,
        x: c_double,
        y: c_double,
        xTolerance: c_double,
        yTolerance: c_double,
    ) -> c_int;

    #[doc = " Function: FPDFText_GetText\n          Extract unicode text string from the page.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          start_index -   Index for the start characters.\n          count       -   Number of UCS-2 values to be extracted.\n          result      -   A buffer (allocated by application) receiving the\n                          extracted UCS-2 values. The buffer must be able to\n                          hold `count` UCS-2 values plus a terminator.\n Return Value:\n          Number of characters written into the result buffer, including the\n          trailing terminator.\n Comments:\n          This function ignores characters without UCS-2 representations.\n          It considers all characters on the page, even those that are not\n          visible when the page has a cropbox. To filter out the characters\n          outside of the cropbox, use FPDF_GetPageBoundingBox() and\n          FPDFText_GetCharBox().\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetText(
        &self,
        text_page: FPDF_TEXTPAGE,
        start_index: c_int,
        count: c_int,
        result: *mut c_ushort,
    ) -> c_int;

    #[doc = " Function: FPDFText_CountRects\n          Counts number of rectangular areas occupied by a segment of text,\n          and caches the result for subsequent FPDFText_GetRect() calls.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          start_index -   Index for the start character.\n          count       -   Number of characters, or -1 for all remaining.\n Return value:\n          Number of rectangles, 0 if text_page is null, or -1 on bad\n          start_index.\n Comments:\n          This function, along with FPDFText_GetRect can be used by\n          applications to detect the position on the page for a text segment,\n          so proper areas can be highlighted. The FPDFText_* functions will\n          automatically merge small character boxes into bigger one if those\n          characters are on the same line and use same font settings.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_CountRects(
        &self,
        text_page: FPDF_TEXTPAGE,
        start_index: c_int,
        count: c_int,
    ) -> c_int;

    #[doc = " Function: FPDFText_GetRect\n          Get a rectangular area from the result generated by\n          FPDFText_CountRects.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          rect_index  -   Zero-based index for the rectangle.\n          left        -   Pointer to a double value receiving the rectangle\n                          left boundary.\n          top         -   Pointer to a double value receiving the rectangle\n                          top boundary.\n          right       -   Pointer to a double value receiving the rectangle\n                          right boundary.\n          bottom      -   Pointer to a double value receiving the rectangle\n                          bottom boundary.\n Return Value:\n          On success, return TRUE and fill in |left|, |top|, |right|, and\n          |bottom|. If |text_page| is invalid then return FALSE, and the out\n          parameters remain unmodified. If |text_page| is valid but\n          |rect_index| is out of bounds, then return FALSE and set the out\n          parameters to 0.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetRect(
        &self,
        text_page: FPDF_TEXTPAGE,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL;

    #[doc = " Function: FPDFText_GetBoundedText\n          Extract unicode text within a rectangular boundary on the page.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          left        -   Left boundary.\n          top         -   Top boundary.\n          right       -   Right boundary.\n          bottom      -   Bottom boundary.\n          buffer      -   Caller-allocated buffer to receive UTF-16 values.\n          buflen      -   Number of UTF-16 values (not bytes) that `buffer`\n                          is capable of holding.\n Return Value:\n          If buffer is NULL or buflen is zero, return number of UTF-16\n          values (not bytes) of text present within the rectangle, excluding\n          a terminating NUL. Generally you should pass a buffer at least one\n          larger than this if you want a terminating NUL, which will be\n          provided if space is available. Otherwise, return number of UTF-16\n          values copied into the buffer, including the terminating NUL when\n          space for it is available.\n Comment:\n          If the buffer is too small, as much text as will fit is copied into\n          it. May return a split surrogate in that case.\n"]
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFText_GetBoundedText(
        &self,
        text_page: FPDF_TEXTPAGE,
        left: c_double,
        top: c_double,
        right: c_double,
        bottom: c_double,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int;

    #[doc = " Function: FPDFText_FindStart\n          Start a search.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n          findwhat    -   A unicode match pattern.\n          flags       -   Option flags.\n          start_index -   Start from this character. -1 for end of the page.\n Return Value:\n          A handle for the search context. FPDFText_FindClose must be called\n          to release this handle.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_FindStart(
        &self,
        text_page: FPDF_TEXTPAGE,
        findwhat: FPDF_WIDESTRING,
        flags: c_ulong,
        start_index: c_int,
    ) -> FPDF_SCHHANDLE;

    // TODO: AJRC - 24-Aug-24 - need doc comment for helper function
    #[allow(non_snake_case)]
    fn FPDFText_FindStart_str(
        &self,
        text_page: FPDF_TEXTPAGE,
        findwhat: &str,
        flags: c_ulong,
        start_index: c_int,
    ) -> FPDF_SCHHANDLE {
        self.FPDFText_FindStart(
            text_page,
            get_pdfium_utf16le_bytes_from_str(findwhat).as_ptr() as FPDF_WIDESTRING,
            flags,
            start_index,
        )
    }

    #[doc = " Function: FPDFText_FindNext\n          Search in the direction from page start to end.\n Parameters:\n          handle      -   A search context handle returned by\n                          FPDFText_FindStart.\n Return Value:\n          Whether a match is found.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_FindNext(&self, handle: FPDF_SCHHANDLE) -> FPDF_BOOL;

    #[doc = " Function: FPDFText_FindPrev\n          Search in the direction from page end to start.\n Parameters:\n          handle      -   A search context handle returned by\n                          FPDFText_FindStart.\n Return Value:\n          Whether a match is found.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_FindPrev(&self, handle: FPDF_SCHHANDLE) -> FPDF_BOOL;

    #[doc = " Function: FPDFText_GetSchResultIndex\n          Get the starting character index of the search result.\n Parameters:\n          handle      -   A search context handle returned by\n                          FPDFText_FindStart.\n Return Value:\n          Index for the starting character.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetSchResultIndex(&self, handle: FPDF_SCHHANDLE) -> c_int;

    #[doc = " Function: FPDFText_GetSchCount\n          Get the number of matched characters in the search result.\n Parameters:\n          handle      -   A search context handle returned by\n                          FPDFText_FindStart.\n Return Value:\n          Number of matched characters.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_GetSchCount(&self, handle: FPDF_SCHHANDLE) -> c_int;

    #[doc = " Function: FPDFText_FindClose\n          Release a search context.\n Parameters:\n          handle      -   A search context handle returned by\n                          FPDFText_FindStart.\n Return Value:\n          None.\n"]
    #[allow(non_snake_case)]
    fn FPDFText_FindClose(&self, handle: FPDF_SCHHANDLE);

    #[doc = " Function: FPDFLink_LoadWebLinks\n          Prepare information about weblinks in a page.\n Parameters:\n          text_page   -   Handle to a text page information structure.\n                          Returned by FPDFText_LoadPage function.\n Return Value:\n          A handle to the page's links information structure, or\n          NULL if something goes wrong.\n Comments:\n          Weblinks are those links implicitly embedded in PDF pages. PDF also\n          has a type of annotation called \"link\" (FPDFTEXT doesn't deal with\n          that kind of link). FPDFTEXT weblink feature is useful for\n          automatically detecting links in the page contents. For example,\n          things like \"https://www.example.com\" will be detected, so\n          applications can allow user to click on those characters to activate\n          the link, even the PDF doesn't come with link annotations.\n\n          FPDFLink_CloseWebLinks must be called to release resources.\n"]
    #[allow(non_snake_case)]
    fn FPDFLink_LoadWebLinks(&self, text_page: FPDF_TEXTPAGE) -> FPDF_PAGELINK;

    #[doc = " Function: FPDFLink_CountWebLinks\n          Count number of detected web links.\n Parameters:\n          link_page   -   Handle returned by FPDFLink_LoadWebLinks.\n Return Value:\n          Number of detected web links.\n"]
    #[allow(non_snake_case)]
    fn FPDFLink_CountWebLinks(&self, link_page: FPDF_PAGELINK) -> c_int;

    #[doc = " Function: FPDFLink_GetURL\n          Fetch the URL information for a detected web link.\n Parameters:\n          link_page   -   Handle returned by FPDFLink_LoadWebLinks.\n          link_index  -   Zero-based index for the link.\n          buffer      -   A unicode buffer for the result.\n          buflen      -   Number of 16-bit code units (not bytes) for the\n                          buffer, including an additional terminator.\n Return Value:\n          If |buffer| is NULL or |buflen| is zero, return the number of 16-bit\n          code units (not bytes) needed to buffer the result (an additional\n          terminator is included in this count).\n          Otherwise, copy the result into |buffer|, truncating at |buflen| if\n          the result is too large to fit, and return the number of 16-bit code\n          units actually copied into the buffer (the additional terminator is\n          also included in this count).\n          If |link_index| does not correspond to a valid link, then the result\n          is an empty string.\n"]
    #[allow(non_snake_case)]
    fn FPDFLink_GetURL(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        buffer: *mut c_ushort,
        buflen: c_int,
    ) -> c_int;

    #[doc = " Function: FPDFLink_CountRects\n          Count number of rectangular areas for the link.\n Parameters:\n          link_page   -   Handle returned by FPDFLink_LoadWebLinks.\n          link_index  -   Zero-based index for the link.\n Return Value:\n          Number of rectangular areas for the link.  If |link_index| does\n          not correspond to a valid link, then 0 is returned.\n"]
    #[allow(non_snake_case)]
    fn FPDFLink_CountRects(&self, link_page: FPDF_PAGELINK, link_index: c_int) -> c_int;

    #[doc = " Function: FPDFLink_GetRect\n          Fetch the boundaries of a rectangle for a link.\n Parameters:\n          link_page   -   Handle returned by FPDFLink_LoadWebLinks.\n          link_index  -   Zero-based index for the link.\n          rect_index  -   Zero-based index for a rectangle.\n          left        -   Pointer to a double value receiving the rectangle\n                          left boundary.\n          top         -   Pointer to a double value receiving the rectangle\n                          top boundary.\n          right       -   Pointer to a double value receiving the rectangle\n                          right boundary.\n          bottom      -   Pointer to a double value receiving the rectangle\n                          bottom boundary.\n Return Value:\n          On success, return TRUE and fill in |left|, |top|, |right|, and\n          |bottom|. If |link_page| is invalid or if |link_index| does not\n          correspond to a valid link, then return FALSE, and the out\n          parameters remain unmodified.\n"]
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFLink_GetRect(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        rect_index: c_int,
        left: *mut c_double,
        top: *mut c_double,
        right: *mut c_double,
        bottom: *mut c_double,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Function: FPDFLink_GetTextRange\n          Fetch the start char index and char count for a link.\n Parameters:\n          link_page         -   Handle returned by FPDFLink_LoadWebLinks.\n          link_index        -   Zero-based index for the link.\n          start_char_index  -   pointer to int receiving the start char index\n          char_count        -   pointer to int receiving the char count\n Return Value:\n          On success, return TRUE and fill in |start_char_index| and\n          |char_count|. if |link_page| is invalid or if |link_index| does\n          not correspond to a valid link, then return FALSE and the out\n          parameters remain unmodified.\n"]
    #[allow(non_snake_case)]
    fn FPDFLink_GetTextRange(
        &self,
        link_page: FPDF_PAGELINK,
        link_index: c_int,
        start_char_index: *mut c_int,
        char_count: *mut c_int,
    ) -> FPDF_BOOL;

    #[doc = " Function: FPDFLink_CloseWebLinks\n          Release resources used by weblink feature.\n Parameters:\n          link_page   -   Handle returned by FPDFLink_LoadWebLinks.\n Return Value:\n          None.\n"]
    #[allow(non_snake_case)]

    fn FPDFLink_CloseWebLinks(&self, link_page: FPDF_PAGELINK);

    #[allow(non_snake_case)]
    fn FPDFPage_GetDecodedThumbnailData(
        &self,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    #[allow(non_snake_case)]
    fn FPDFPage_GetRawThumbnailData(
        &self,
        page: FPDF_PAGE,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    #[allow(non_snake_case)]
    fn FPDFPage_GetThumbnailAsBitmap(&self, page: FPDF_PAGE) -> FPDF_BITMAP;

    #[allow(non_snake_case)]
    fn FPDFFormObj_CountObjects(&self, form_object: FPDF_PAGEOBJECT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFFormObj_GetObject(
        &self,
        form_object: FPDF_PAGEOBJECT,
        index: c_ulong,
    ) -> FPDF_PAGEOBJECT;

    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateTextObj(
        &self,
        document: FPDF_DOCUMENT,
        font: FPDF_FONT,
        font_size: c_float,
    ) -> FPDF_PAGEOBJECT;

    #[allow(non_snake_case)]
    fn FPDFTextObj_GetTextRenderMode(&self, text: FPDF_PAGEOBJECT) -> FPDF_TEXT_RENDERMODE;

    #[allow(non_snake_case)]
    fn FPDFTextObj_SetTextRenderMode(
        &self,
        text: FPDF_PAGEOBJECT,
        render_mode: FPDF_TEXT_RENDERMODE,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFTextObj_GetText(
        &self,
        text_object: FPDF_PAGEOBJECT,
        text_page: FPDF_TEXTPAGE,
        buffer: *mut FPDF_WCHAR,
        length: c_ulong,
    ) -> c_ulong;

    #[doc = " Experimental API.\n Get a bitmap rasterization of |text_object|. To render correctly, the caller\n must provide the |document| associated with |text_object|. If there is a\n |page| associated with |text_object|, the caller should provide that as well.\n The returned bitmap will be owned by the caller, and FPDFBitmap_Destroy()\n must be called on the returned bitmap when it is no longer needed.\n\n   document    - handle to a document associated with |text_object|.\n   page        - handle to an optional page associated with |text_object|.\n   text_object - handle to a text object.\n   scale       - the scaling factor, which must be greater than 0.\n\n Returns the bitmap or NULL on failure."]
    #[allow(non_snake_case)]
    fn FPDFTextObj_GetRenderedBitmap(
        &self,
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        text_object: FPDF_PAGEOBJECT,
        scale: f32,
    ) -> FPDF_BITMAP;

    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFont(&self, text: FPDF_PAGEOBJECT) -> FPDF_FONT;

    #[allow(non_snake_case)]
    fn FPDFTextObj_GetFontSize(&self, text: FPDF_PAGEOBJECT, size: *mut c_float) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFFont_Close(&self, font: FPDF_FONT);

    #[allow(non_snake_case)]
    fn FPDFPath_MoveTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPath_LineTo(&self, path: FPDF_PAGEOBJECT, x: c_float, y: c_float) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFPath_BezierTo(
        &self,
        path: FPDF_PAGEOBJECT,
        x1: c_float,
        y1: c_float,
        x2: c_float,
        y2: c_float,
        x3: c_float,
        y3: c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPath_Close(&self, path: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPath_SetDrawMode(
        &self,
        path: FPDF_PAGEOBJECT,
        fillmode: c_int,
        stroke: FPDF_BOOL,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPath_GetDrawMode(
        &self,
        path: FPDF_PAGEOBJECT,
        fillmode: *mut c_int,
        stroke: *mut FPDF_BOOL,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_NewTextObj(
        &self,
        document: FPDF_DOCUMENT,
        font: &str,
        font_size: c_float,
    ) -> FPDF_PAGEOBJECT;

    #[allow(non_snake_case)]
    fn FPDFText_SetText(&self, text_object: FPDF_PAGEOBJECT, text: FPDF_WIDESTRING) -> FPDF_BOOL;

    #[inline]
    #[allow(non_snake_case)]
    fn FPDFText_SetText_str(&self, text_object: FPDF_PAGEOBJECT, text: &str) -> FPDF_BOOL {
        self.FPDFText_SetText(
            text_object,
            get_pdfium_utf16le_bytes_from_str(text).as_ptr() as FPDF_WIDESTRING,
        )
    }

    #[doc = " Experimental API.\n Set the text using charcodes for a text object. If it had text, it will be\n replaced.\n\n text_object  - handle to the text object.\n charcodes    - pointer to an array of charcodes to be added.\n count        - number of elements in |charcodes|.\n\n Returns TRUE on success"]
    #[allow(non_snake_case)]
    fn FPDFText_SetCharcodes(
        &self,
        text_object: FPDF_PAGEOBJECT,
        charcodes: *const c_uint,
        count: size_t,
    ) -> FPDF_BOOL;

    #[doc = " Returns a font object loaded from a stream of data. The font is loaded\n into the document. Various font data structures, such as the ToUnicode data,\n are auto-generated based on the inputs.\n\n document  - handle to the document.\n data      - the stream of font data, which will be copied by the font object.\n size      - the size of the font data, in bytes.\n font_type - FPDF_FONT_TYPE1 or FPDF_FONT_TRUETYPE depending on the font type.\n cid       - a boolean specifying if the font is a CID font or not.\n\n The loaded font can be closed using FPDFFont_Close().\n\n Returns NULL on failure"]
    #[allow(non_snake_case)]
    fn FPDFText_LoadFont(
        &self,
        document: FPDF_DOCUMENT,
        data: *const c_uchar,
        size: c_uint,
        font_type: c_int,
        cid: FPDF_BOOL,
    ) -> FPDF_FONT;

    #[doc = " Experimental API.\n Loads one of the standard 14 fonts per PDF spec 1.7 page 416. The preferred\n way of using font style is using a dash to separate the name from the style,\n for example 'Helvetica-BoldItalic'.\n\n document   - handle to the document.\n font       - string containing the font name, without spaces.\n\n The loaded font can be closed using FPDFFont_Close().\n\n Returns NULL on failure."]
    #[allow(non_snake_case)]
    fn FPDFText_LoadStandardFont(&self, document: FPDF_DOCUMENT, font: &str) -> FPDF_FONT;

    #[cfg(any(
        feature = "pdfium_6295",
        feature = "pdfium_6337",
        feature = "pdfium_6406",
        feature = "pdfium_6490",
        feature = "pdfium_6555",
        feature = "pdfium_6569",
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    #[doc = " Experimental API.\n Returns a font object loaded from a stream of data for a type 2 CID font. The\n font is loaded into the document. Unlike FPDFText_LoadFont(), the ToUnicode\n data and the CIDToGIDMap data are caller provided, instead of auto-generated.\n\n document                 - handle to the document.\n font_data                - the stream of font data, which will be copied by\n                            the font object.\n font_data_size           - the size of the font data, in bytes.\n to_unicode_cmap          - the ToUnicode data.\n cid_to_gid_map_data      - the stream of CIDToGIDMap data.\n cid_to_gid_map_data_size - the size of the CIDToGIDMap data, in bytes.\n\n The loaded font can be closed using FPDFFont_Close().\n\n Returns NULL on failure."]
    #[allow(non_snake_case)]
    fn FPDFText_LoadCidType2Font(
        &self,
        document: FPDF_DOCUMENT,
        font_data: *const u8,
        font_data_size: u32,
        to_unicode_cmap: &str,
        cid_to_gid_map_data: *const u8,
        cid_to_gid_map_data_size: u32,
    ) -> FPDF_FONT;

    #[allow(non_snake_case)]
    fn FPDFPage_InsertObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT);

    #[allow(non_snake_case)]
    fn FPDFPage_RemoveObject(&self, page: FPDF_PAGE, page_obj: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPage_CountObjects(&self, page: FPDF_PAGE) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPage_GetObject(&self, page: FPDF_PAGE, index: c_int) -> FPDF_PAGEOBJECT;

    #[allow(non_snake_case)]
    fn FPDFPageObj_Destroy(&self, page_obj: FPDF_PAGEOBJECT);

    #[allow(non_snake_case)]
    fn FPDFPageObj_HasTransparency(&self, page_object: FPDF_PAGEOBJECT) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetType(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    fn FPDFPageObj_Transform(
        &self,
        page_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    );

    #[cfg(any(
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Transform `page_object` by the given matrix.
    ///
    ///   `page_object` - handle to a page object.
    ///
    ///   `matrix`      - the transform matrix.
    ///
    /// Returns `TRUE` on success.
    ///
    /// This can be used to scale, rotate, shear and translate the `page_object`.
    /// It is an improved version of [PdfiumLibraryBindings::FPDFPageObj_Transform]
    /// that does not do unnecessary double to float conversions, and only uses 1 parameter
    /// for the matrix. It also returns whether the operation succeeded or not.
    #[allow(non_snake_case)]
    fn FPDFPageObj_TransformF(
        &self,
        page_object: FPDF_PAGEOBJECT,
        matrix: *const FS_MATRIX,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMatrix(
        &self,
        page_object: FPDF_PAGEOBJECT,
        matrix: *mut FS_MATRIX,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetMatrix(&self, path: FPDF_PAGEOBJECT, matrix: *const FS_MATRIX) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_NewImageObj(&self, document: FPDF_DOCUMENT) -> FPDF_PAGEOBJECT;

    #[cfg(any(
        feature = "pdfium_6611",
        feature = "pdfium_6666",
        feature = "pdfium_future"
    ))]
    /// Get the marked content ID for the object.
    ///
    ///   `page_object` - handle to a page object.
    ///
    /// Returns the page object's marked content ID, or -1 on error.
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMarkedContentID(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPageObj_CountMarks(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetMark(
        &self,
        page_object: FPDF_PAGEOBJECT,
        index: c_ulong,
    ) -> FPDF_PAGEOBJECTMARK;

    #[allow(non_snake_case)]
    fn FPDFPageObj_AddMark(&self, page_object: FPDF_PAGEOBJECT, name: &str) -> FPDF_PAGEOBJECTMARK;

    #[allow(non_snake_case)]
    fn FPDFPageObj_RemoveMark(
        &self,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetName(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_CountParams(&self, mark: FPDF_PAGEOBJECTMARK) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamKey(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        index: c_ulong,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamValueType(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
    ) -> FPDF_OBJECT_TYPE;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamIntValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        out_value: *mut c_int,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamStringValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_GetParamBlobValue(
        &self,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetIntParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: c_int,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetStringParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: &str,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_SetBlobParam(
        &self,
        document: FPDF_DOCUMENT,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
        value: *mut c_void,
        value_len: c_ulong,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObjMark_RemoveParam(
        &self,
        page_object: FPDF_PAGEOBJECT,
        mark: FPDF_PAGEOBJECTMARK,
        key: &str,
    ) -> FPDF_BOOL;

    #[doc = " Load an image from a JPEG image file and then set it into |image_object|.\n\n   pages        - pointer to the start of all loaded pages, may be NULL.\n   count        - number of |pages|, may be 0.\n   image_object - handle to an image object.\n   file_access  - file access handler which specifies the JPEG image file.\n\n Returns TRUE on success.\n\n The image object might already have an associated image, which is shared and\n cached by the loaded pages. In that case, we need to clear the cached image\n for all the loaded pages. Pass |pages| and page count (|count|) to this API\n to clear the image cache. If the image is not previously shared, or NULL is a\n valid |pages| value."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_LoadJpegFile(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL;

    #[doc = " Load an image from a JPEG image file and then set it into |image_object|.\n\n   pages        - pointer to the start of all loaded pages, may be NULL.\n   count        - number of |pages|, may be 0.\n   image_object - handle to an image object.\n   file_access  - file access handler which specifies the JPEG image file.\n\n Returns TRUE on success.\n\n The image object might already have an associated image, which is shared and\n cached by the loaded pages. In that case, we need to clear the cached image\n for all the loaded pages. Pass |pages| and page count (|count|) to this API\n to clear the image cache. If the image is not previously shared, or NULL is a\n valid |pages| value. This function loads the JPEG image inline, so the image\n content is copied to the file. This allows |file_access| and its associated\n data to be deleted after this function returns."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_LoadJpegFileInline(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        file_access: *mut FPDF_FILEACCESS,
    ) -> FPDF_BOOL;

    #[doc = " TODO(thestig): Start deprecating this once FPDFPageObj_SetMatrix() is stable.\n\n Set the transform matrix of |image_object|.\n\n   image_object - handle to an image object.\n   a            - matrix value.\n   b            - matrix value.\n   c            - matrix value.\n   d            - matrix value.\n   e            - matrix value.\n   f            - matrix value.\n\n The matrix is composed as:\n   |a c e|\n   |b d f|\n and can be used to scale, rotate, shear and translate the |image_object|.\n\n Returns TRUE on success."]
    #[allow(non_snake_case)]
    #[allow(clippy::too_many_arguments)]
    #[deprecated(
        note = "Prefer FPDFPageObj_SetMatrix() over FPDFImageObj_SetMatrix(). FPDFImageObj_SetMatrix() is deprecated and will likely be removed in a future version of Pdfium."
    )]
    fn FPDFImageObj_SetMatrix(
        &self,
        image_object: FPDF_PAGEOBJECT,
        a: c_double,
        b: c_double,
        c: c_double,
        d: c_double,
        e: c_double,
        f: c_double,
    ) -> FPDF_BOOL;

    #[doc = " Set |bitmap| to |image_object|.\n\n   pages        - pointer to the start of all loaded pages, may be NULL.\n   count        - number of |pages|, may be 0.\n   image_object - handle to an image object.\n   bitmap       - handle of the bitmap.\n\n Returns TRUE on success."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_SetBitmap(
        &self,
        pages: *mut FPDF_PAGE,
        count: c_int,
        image_object: FPDF_PAGEOBJECT,
        bitmap: FPDF_BITMAP,
    ) -> FPDF_BOOL;

    #[doc = " Get a bitmap rasterization of |image_object|. FPDFImageObj_GetBitmap() only\n operates on |image_object| and does not take the associated image mask into\n account. It also ignores the matrix for |image_object|.\n The returned bitmap will be owned by the caller, and FPDFBitmap_Destroy()\n must be called on the returned bitmap when it is no longer needed.\n\n   image_object - handle to an image object.\n\n Returns the bitmap."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetBitmap(&self, image_object: FPDF_PAGEOBJECT) -> FPDF_BITMAP;

    #[doc = " Experimental API.\n Get a bitmap rasterization of |image_object| that takes the image mask and\n image matrix into account. To render correctly, the caller must provide the\n |document| associated with |image_object|. If there is a |page| associated\n with |image_object|, the caller should provide that as well.\n The returned bitmap will be owned by the caller, and FPDFBitmap_Destroy()\n must be called on the returned bitmap when it is no longer needed.\n\n   document     - handle to a document associated with |image_object|.\n   page         - handle to an optional page associated with |image_object|.\n   image_object - handle to an image object.\n\n Returns the bitmap or NULL on failure."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetRenderedBitmap(
        &self,
        document: FPDF_DOCUMENT,
        page: FPDF_PAGE,
        image_object: FPDF_PAGEOBJECT,
    ) -> FPDF_BITMAP;

    #[doc = " Get the decoded image data of |image_object|. The decoded data is the\n uncompressed image data, i.e. the raw image data after having all filters\n applied. |buffer| is only modified if |buflen| is longer than the length of\n the decoded image data.\n\n   image_object - handle to an image object.\n   buffer       - buffer for holding the decoded image data.\n   buflen       - length of the buffer in bytes.\n\n Returns the length of the decoded image data."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataDecoded(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    #[doc = " Get the raw image data of |image_object|. The raw data is the image data as\n stored in the PDF without applying any filters. |buffer| is only modified if\n |buflen| is longer than the length of the raw image data.\n\n   image_object - handle to an image object.\n   buffer       - buffer for holding the raw image data.\n   buflen       - length of the buffer in bytes.\n\n Returns the length of the raw image data."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageDataRaw(
        &self,
        image_object: FPDF_PAGEOBJECT,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    #[doc = " Get the number of filters (i.e. decoders) of the image in |image_object|.\n\n   image_object - handle to an image object.\n\n Returns the number of |image_object|'s filters."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageFilterCount(&self, image_object: FPDF_PAGEOBJECT) -> c_int;

    #[doc = " Get the filter at |index| of |image_object|'s list of filters. Note that the\n filters need to be applied in order, i.e. the first filter should be applied\n first, then the second, etc. |buffer| is only modified if |buflen| is longer\n than the length of the filter string.\n\n   image_object - handle to an image object.\n   index        - the index of the filter requested.\n   buffer       - buffer for holding filter string, encoded in UTF-8.\n   buflen       - length of the buffer.\n\n Returns the length of the filter string."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageFilter(
        &self,
        image_object: FPDF_PAGEOBJECT,
        index: c_int,
        buffer: *mut c_void,
        buflen: c_ulong,
    ) -> c_ulong;

    #[doc = " Get the image metadata of |image_object|, including dimension, DPI, bits per\n pixel, and colorspace. If the |image_object| is not an image object or if it\n does not have an image, then the return value will be false. Otherwise,\n failure to retrieve any specific parameter would result in its value being 0.\n\n   image_object - handle to an image object.\n   page         - handle to the page that |image_object| is on. Required for\n                  retrieving the image's bits per pixel and colorspace.\n   metadata     - receives the image metadata; must not be NULL.\n\n Returns true if successful."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImageMetadata(
        &self,
        image_object: FPDF_PAGEOBJECT,
        page: FPDF_PAGE,
        metadata: *mut FPDF_IMAGEOBJ_METADATA,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Get the image size in pixels. Faster method to get only image size.\n\n   image_object - handle to an image object.\n   width        - receives the image width in pixels; must not be NULL.\n   height       - receives the image height in pixels; must not be NULL.\n\n Returns true if successful."]
    #[allow(non_snake_case)]
    fn FPDFImageObj_GetImagePixelSize(
        &self,
        image_object: FPDF_PAGEOBJECT,
        width: *mut c_uint,
        height: *mut c_uint,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewPath(&self, x: c_float, y: c_float) -> FPDF_PAGEOBJECT;

    #[allow(non_snake_case)]
    fn FPDFPageObj_CreateNewRect(
        &self,
        x: c_float,
        y: c_float,
        w: c_float,
        h: c_float,
    ) -> FPDF_PAGEOBJECT;

    #[doc = " Get the bounding box of |page_object|.\n\n page_object  - handle to a page object.\n left         - pointer where the left coordinate will be stored\n bottom       - pointer where the bottom coordinate will be stored\n right        - pointer where the right coordinate will be stored\n top          - pointer where the top coordinate will be stored\n\n On success, returns TRUE and fills in the 4 coordinates."]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetBounds(
        &self,
        page_object: FPDF_PAGEOBJECT,
        left: *mut c_float,
        bottom: *mut c_float,
        right: *mut c_float,
        top: *mut c_float,
    ) -> FPDF_BOOL;

    #[doc = " Experimental API.\n Get the quad points that bounds |page_object|.\n\n page_object  - handle to a page object.\n quad_points  - pointer where the quadrilateral points will be stored.\n\n On success, returns TRUE and fills in |quad_points|.\n\n Similar to FPDFPageObj_GetBounds(), this returns the bounds of a page\n object. When the object is rotated by a non-multiple of 90 degrees, this API\n returns a tighter bound that cannot be represented with just the 4 sides of\n a rectangle.\n\n Currently only works the following |page_object| types: FPDF_PAGEOBJ_TEXT and\n FPDF_PAGEOBJ_IMAGE."]
    #[allow(non_snake_case)]
    fn FPDFPageObj_GetRotatedBounds(
        &self,
        page_object: FPDF_PAGEOBJECT,
        quad_points: *mut FS_QUADPOINTSF,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetBlendMode(&self, page_object: FPDF_PAGEOBJECT, blend_mode: &str);

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetStrokeColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetStrokeColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetStrokeWidth(&self, page_object: FPDF_PAGEOBJECT, width: c_float)
        -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetStrokeWidth(
        &self,
        page_object: FPDF_PAGEOBJECT,
        width: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineJoin(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineJoin(&self, page_object: FPDF_PAGEOBJECT, line_join: c_int) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetLineCap(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetLineCap(&self, page_object: FPDF_PAGEOBJECT, line_cap: c_int) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: c_uint,
        G: c_uint,
        B: c_uint,
        A: c_uint,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetFillColor(
        &self,
        page_object: FPDF_PAGEOBJECT,
        R: *mut c_uint,
        G: *mut c_uint,
        B: *mut c_uint,
        A: *mut c_uint,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashPhase(
        &self,
        page_object: FPDF_PAGEOBJECT,
        phase: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetDashPhase(&self, page_object: FPDF_PAGEOBJECT, phase: c_float) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashCount(&self, page_object: FPDF_PAGEOBJECT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPageObj_GetDashArray(
        &self,
        page_object: FPDF_PAGEOBJECT,
        dash_array: *mut c_float,
        dash_count: size_t,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPageObj_SetDashArray(
        &self,
        page_object: FPDF_PAGEOBJECT,
        dash_array: *const c_float,
        dash_count: size_t,
        phase: c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPath_CountSegments(&self, path: FPDF_PAGEOBJECT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPath_GetPathSegment(&self, path: FPDF_PAGEOBJECT, index: c_int) -> FPDF_PATHSEGMENT;

    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetPoint(
        &self,
        segment: FPDF_PATHSEGMENT,
        x: *mut c_float,
        y: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetType(&self, segment: FPDF_PATHSEGMENT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFPathSegment_GetClose(&self, segment: FPDF_PATHSEGMENT) -> FPDF_BOOL;

    // TODO: AJRC - 4-Aug-2024 - FPDFFont_GetBaseFontName() is in Pdfium export headers
    // but changes not yet released. Tracking issue: https://github.com/ajrcarey/pdfium-render/issues/152
    #[cfg(any(feature = "pdfium_6666", feature = "pdfium_future"))]
    #[allow(non_snake_case)]
    fn FPDFFont_GetBaseFontName(
        &self,
        font: FPDF_FONT,
        buffer: *mut c_char,
        length: size_t,
    ) -> size_t;

    // TODO: AJRC - 4-Aug-2024 - pointer type updated in FPDFFont_GetBaseFontName() definition,
    // but changes not yet released. Tracking issue: https://github.com/ajrcarey/pdfium-render/issues/152
    #[cfg(any(feature = "pdfium_6666", feature = "pdfium_future"))]
    #[allow(non_snake_case)]
    fn FPDFFont_GetFamilyName(
        &self,
        font: FPDF_FONT,
        buffer: *mut c_char,
        length: size_t,
    ) -> size_t;

    #[cfg(feature = "pdfium_6611")]
    #[allow(non_snake_case)]
    fn FPDFFont_GetFamilyName(
        &self,
        font: FPDF_FONT,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong;

    #[cfg(any(
        feature = "pdfium_6569",
        feature = "pdfium_6555",
        feature = "pdfium_6490",
        feature = "pdfium_6406",
        feature = "pdfium_6337",
        feature = "pdfium_6295",
        feature = "pdfium_6259",
        feature = "pdfium_6164",
        feature = "pdfium_6124",
        feature = "pdfium_6110",
        feature = "pdfium_6084",
        feature = "pdfium_6043",
        feature = "pdfium_6015",
        feature = "pdfium_5961"
    ))]
    #[allow(non_snake_case)]
    fn FPDFFont_GetFontName(
        &self,
        font: FPDF_FONT,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong;

    #[allow(non_snake_case)]
    fn FPDFFont_GetFontData(
        &self,
        font: FPDF_FONT,
        buffer: *mut u8,
        buflen: size_t,
        out_buflen: *mut size_t,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFFont_GetIsEmbedded(&self, font: FPDF_FONT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFFont_GetFlags(&self, font: FPDF_FONT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFFont_GetWeight(&self, font: FPDF_FONT) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFFont_GetItalicAngle(&self, font: FPDF_FONT, angle: *mut c_int) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFFont_GetAscent(
        &self,
        font: FPDF_FONT,
        font_size: c_float,
        ascent: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFFont_GetDescent(
        &self,
        font: FPDF_FONT,
        font_size: c_float,
        descent: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFFont_GetGlyphWidth(
        &self,
        font: FPDF_FONT,
        glyph: c_uint,
        font_size: c_float,
        width: *mut c_float,
    ) -> FPDF_BOOL;

    #[allow(non_snake_case)]
    fn FPDFFont_GetGlyphPath(
        &self,
        font: FPDF_FONT,
        glyph: c_uint,
        font_size: c_float,
    ) -> FPDF_GLYPHPATH;

    #[allow(non_snake_case)]
    fn FPDFGlyphPath_CountGlyphSegments(&self, glyphpath: FPDF_GLYPHPATH) -> c_int;

    #[allow(non_snake_case)]
    fn FPDFGlyphPath_GetGlyphPathSegment(
        &self,
        glyphpath: FPDF_GLYPHPATH,
        index: c_int,
    ) -> FPDF_PATHSEGMENT;

    #[doc = " Function: FPDF_VIEWERREF_GetPrintScaling\n          Whether the PDF document prefers to be scaled or not.\n Parameters:\n          document    -   Handle to the loaded document.\n Return value:\n          None."]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintScaling(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL;

    #[doc = " Function: FPDF_VIEWERREF_GetNumCopies\n          Returns the number of copies to be printed.\n Parameters:\n          document    -   Handle to the loaded document.\n Return value:\n          The number of copies to be printed."]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetNumCopies(&self, document: FPDF_DOCUMENT) -> c_int;

    #[doc = " Function: FPDF_VIEWERREF_GetPrintPageRange\n          Page numbers to initialize print dialog box when file is printed.\n Parameters:\n          document    -   Handle to the loaded document.\n Return value:\n          The print page range to be used for printing."]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRange(&self, document: FPDF_DOCUMENT) -> FPDF_PAGERANGE;

    #[doc = " Experimental API.\n Function: FPDF_VIEWERREF_GetPrintPageRangeCount\n          Returns the number of elements in a FPDF_PAGERANGE.\n Parameters:\n          pagerange   -   Handle to the page range.\n Return value:\n          The number of elements in the page range. Returns 0 on error."]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeCount(&self, pagerange: FPDF_PAGERANGE) -> size_t;

    #[doc = " Experimental API.\n Function: FPDF_VIEWERREF_GetPrintPageRangeElement\n          Returns an element from a FPDF_PAGERANGE.\n Parameters:\n          pagerange   -   Handle to the page range.\n          index       -   Index of the element.\n Return value:\n          The value of the element in the page range at a given index.\n          Returns -1 on error."]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetPrintPageRangeElement(
        &self,
        pagerange: FPDF_PAGERANGE,
        index: size_t,
    ) -> c_int;

    #[doc = " Function: FPDF_VIEWERREF_GetDuplex\n          Returns the paper handling option to be used when printing from\n          the print dialog.\n Parameters:\n          document    -   Handle to the loaded document.\n Return value:\n          The paper handling option to be used when printing."]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetDuplex(&self, document: FPDF_DOCUMENT) -> FPDF_DUPLEXTYPE;

    #[doc = " Function: FPDF_VIEWERREF_GetName\n          Gets the contents for a viewer ref, with a given key. The value must\n          be of type \"name\".\n Parameters:\n          document    -   Handle to the loaded document.\n          key         -   Name of the key in the viewer pref dictionary,\n                          encoded in UTF-8.\n          buffer      -   Caller-allocate buffer to receive the key, or NULL\n                      -   to query the required length.\n          length      -   Length of the buffer.\n Return value:\n          The number of bytes in the contents, including the NULL terminator.\n          Thus if the return value is 0, then that indicates an error, such\n          as when |document| is invalid. If |length| is less than the required\n          length, or |buffer| is NULL, |buffer| will not be modified."]
    #[allow(non_snake_case)]
    fn FPDF_VIEWERREF_GetName(
        &self,
        document: FPDF_DOCUMENT,
        key: &str,
        buffer: *mut c_char,
        length: c_ulong,
    ) -> c_ulong;

    #[doc = " Function: FPDF_CountNamedDests\n          Get the count of named destinations in the PDF document.\n Parameters:\n          document    -   Handle to a document\n Return value:\n          The count of named destinations."]
    #[allow(non_snake_case)]
    fn FPDF_CountNamedDests(&self, document: FPDF_DOCUMENT) -> FPDF_DWORD;

    #[doc = " Function: FPDF_GetNamedDestByName\n          Get a the destination handle for the given name.\n Parameters:\n          document    -   Handle to the loaded document.\n          name        -   The name of a destination.\n Return value:\n          The handle to the destination."]
    #[allow(non_snake_case)]
    fn FPDF_GetNamedDestByName(&self, document: FPDF_DOCUMENT, name: &str) -> FPDF_DEST;

    #[doc = " Function: FPDF_GetNamedDest\n          Get the named destination by index.\n Parameters:\n          document        -   Handle to a document\n          index           -   The index of a named destination.\n          buffer          -   The buffer to store the destination name,\n                              used as wchar_t*.\n          buflen [in/out] -   Size of the buffer in bytes on input,\n                              length of the result in bytes on output\n                              or -1 if the buffer is too small.\n Return value:\n          The destination handle for a given index, or NULL if there is no\n          named destination corresponding to |index|.\n Comments:\n          Call this function twice to get the name of the named destination:\n            1) First time pass in |buffer| as NULL and get buflen.\n            2) Second time pass in allocated |buffer| and buflen to retrieve\n               |buffer|, which should be used as wchar_t*.\n\n         If buflen is not sufficiently large, it will be set to -1 upon\n         return."]
    #[allow(non_snake_case)]
    fn FPDF_GetNamedDest(
        &self,
        document: FPDF_DOCUMENT,
        index: c_int,
        buffer: *mut c_void,
        buflen: *mut c_long,
    ) -> FPDF_DEST;

    /// Get the number of embedded files in `document`.
    ///
    ///   document - handle to a document.
    ///
    /// Returns the number of embedded files in `document`.
    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachmentCount(&self, document: FPDF_DOCUMENT) -> c_int;

    /// Add an embedded file with `name` in `document`. If `name` is empty, or if
    /// `name` is the name of a existing embedded file in `document`, or if
    /// `document`'s embedded file name tree is too deep (i.e. `document` has too
    /// many embedded files already), then a new attachment will not be added.
    ///
    ///   document - handle to a document.
    ///
    ///   name     - name of the new attachment.
    ///
    /// Returns a handle to the new attachment object, or NULL on failure.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFDoc_AddAttachment_str].
    #[allow(non_snake_case)]
    fn FPDFDoc_AddAttachment(
        &self,
        document: FPDF_DOCUMENT,
        name: FPDF_WIDESTRING,
    ) -> FPDF_ATTACHMENT;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFDoc_AddAttachment].
    ///
    /// Add an embedded file with `name` in `document`. If `name` is empty, or if
    /// `name` is the name of a existing embedded file in `document`, or if
    /// `document`'s embedded file name tree is too deep (i.e. `document` has too
    /// many embedded files already), then a new attachment will not be added.
    ///
    ///   document - handle to a document.
    ///
    ///   name     - name of the new attachment.
    ///
    /// Returns a handle to the new attachment object, or NULL on failure.
    #[allow(non_snake_case)]
    fn FPDFDoc_AddAttachment_str(&self, document: FPDF_DOCUMENT, name: &str) -> FPDF_ATTACHMENT {
        self.FPDFDoc_AddAttachment(
            document,
            get_pdfium_utf16le_bytes_from_str(name).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Get the embedded attachment at `index` in `document`. Note that the returned
    /// attachment handle is only valid while `document` is open.
    ///
    ///   document - handle to a document.
    ///
    ///   index    - the index of the requested embedded file.
    ///
    /// Returns the handle to the attachment object, or NULL on failure.
    #[allow(non_snake_case)]
    fn FPDFDoc_GetAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_ATTACHMENT;

    /// Delete the embedded attachment at `index` in `document`. Note that this does
    /// not remove the attachment data from the PDF file; it simply removes the
    /// file's entry in the embedded files name tree so that it does not appear in
    /// the attachment list. This behavior may change in the future.
    ///
    ///   document - handle to a document.
    ///
    ///   index    - the index of the embedded file to be deleted.
    ///
    /// Returns true if successful.
    #[allow(non_snake_case)]
    fn FPDFDoc_DeleteAttachment(&self, document: FPDF_DOCUMENT, index: c_int) -> FPDF_BOOL;

    /// Get the name of the `attachment` file. `buffer` is only modified if `buflen`
    /// is longer than the length of the file name. On errors, `buffer` is unmodified
    /// and the returned length is 0.
    ///
    ///   attachment - handle to an attachment.
    ///
    ///   buffer     - buffer for holding the file name, encoded in UTF-16LE.
    ///
    ///   buflen     - length of the buffer in bytes.
    ///
    /// Returns the length of the file name in bytes.
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetName(
        &self,
        attachment: FPDF_ATTACHMENT,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Check if the params dictionary of `attachment` has `key` as a key.
    ///
    ///   attachment - handle to an attachment.
    ///
    ///   key        - the key to look for, encoded in UTF-8.
    ///
    /// Returns true if `key` exists.
    #[allow(non_snake_case)]
    fn FPDFAttachment_HasKey(&self, attachment: FPDF_ATTACHMENT, key: &str) -> FPDF_BOOL;

    /// Get the type of the value corresponding to `key` in the params dictionary of
    /// the embedded `attachment`.
    ///
    ///   attachment - handle to an attachment.
    ///
    ///   key        - the key to look for, encoded in UTF-8.
    ///
    /// Returns the type of the dictionary value.
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetValueType(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
    ) -> FPDF_OBJECT_TYPE;

    /// Set the string value corresponding to `key` in the params dictionary of the
    /// embedded file `attachment`, overwriting the existing value if any. The value
    /// type should be FPDF_OBJECT_STRING after this function call succeeds.
    ///
    ///   attachment - handle to an attachment.
    ///
    ///   key        - the key to the dictionary entry, encoded in UTF-8.
    ///
    ///   value      - the string value to be set, encoded in UTF-16LE.
    ///
    /// Returns true if successful.
    ///
    /// A [&str]-friendly helper function is available for this function.
    /// See [PdfiumLibraryBindings::FPDFAttachment_SetStringValue_str].
    #[allow(non_snake_case)]
    fn FPDFAttachment_SetStringValue(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
        value: FPDF_WIDESTRING,
    ) -> FPDF_BOOL;

    /// A [&str]-friendly helper function for [PdfiumLibraryBindings::FPDFAttachment_SetStringValue].
    ///
    /// Set the string value corresponding to `key` in the params dictionary of the
    /// embedded file `attachment`, overwriting the existing value if any. The value
    /// type should be FPDF_OBJECT_STRING after this function call succeeds.
    ///
    ///   attachment - handle to an attachment.
    ///
    ///   key        - the key to the dictionary entry.
    ///
    ///   value      - the string value to be set.
    ///
    /// Returns true if successful.
    #[inline]
    #[allow(non_snake_case)]
    fn FPDFAttachment_SetStringValue_str(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
        value: &str,
    ) -> FPDF_BOOL {
        self.FPDFAttachment_SetStringValue(
            attachment,
            key,
            get_pdfium_utf16le_bytes_from_str(value).as_ptr() as FPDF_WIDESTRING,
        )
    }

    /// Get the string value corresponding to `key` in the params dictionary of the
    /// embedded file `attachment`. `buffer` is only modified if `buflen` is longer
    /// than the length of the string value. Note that if `key` does not exist in the
    /// dictionary or if `key`'s corresponding value in the dictionary is not a
    /// string (i.e. the value is not of type FPDF_OBJECT_STRING or
    /// FPDF_OBJECT_NAME), then an empty string would be copied to `buffer` and the
    /// return value would be 2. On other errors, nothing would be added to `buffer`
    /// and the return value would be 0.
    ///
    ///   attachment - handle to an attachment.
    ///
    ///   key        - the key to the requested string value, encoded in UTF-8.
    ///
    ///   buffer     - buffer for holding the string value encoded in UTF-16LE.
    ///
    ///   buflen     - length of the buffer in bytes.
    ///
    /// Returns the length of the dictionary value string in bytes.
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetStringValue(
        &self,
        attachment: FPDF_ATTACHMENT,
        key: &str,
        buffer: *mut FPDF_WCHAR,
        buflen: c_ulong,
    ) -> c_ulong;

    /// Set the file data of `attachment`, overwriting the existing file data if any.
    /// The creation date and checksum will be updated, while all other dictionary
    /// entries will be deleted. Note that only contents with `len` smaller than
    /// INT_MAX is supported.
    ///
    ///   attachment - handle to an attachment.
    ///
    ///   contents   - buffer holding the file data to write to `attachment`.
    ///
    ///   len        - length of file data in bytes.
    ///
    /// Returns true if successful.
    #[allow(non_snake_case)]
    fn FPDFAttachment_SetFile(
        &self,
        attachment: FPDF_ATTACHMENT,
        document: FPDF_DOCUMENT,
        contents: *const c_void,
        len: c_ulong,
    ) -> FPDF_BOOL;

    /// Get the file data of `attachment`.
    ///
    /// When the attachment file data is readable, true is returned, and `out_buflen`
    /// is updated to indicate the file data size. `buffer` is only modified if
    /// `buflen` is non-null and long enough to contain the entire file data. Callers
    /// must check both the return value and the input `buflen` is no less than the
    /// returned `out_buflen` before using the data.
    ///
    /// Otherwise, when the attachment file data is unreadable or when `out_buflen`
    /// is null, false is returned and `buffer` and `out_buflen` remain unmodified.
    ///
    ///   attachment - handle to an attachment.
    ///
    ///   buffer     - buffer for holding the file data from `attachment`.
    ///
    ///   buflen     - length of the buffer in bytes.
    ///
    ///   out_buflen - pointer to the variable that will receive the minimum buffer
    ///                size to contain the file data of `attachment`.
    ///
    /// Returns true on success, false otherwise.
    #[allow(non_snake_case)]
    fn FPDFAttachment_GetFile(
        &self,
        attachment: FPDF_ATTACHMENT,
        buffer: *mut c_void,
        buflen: c_ulong,
        out_buflen: *mut c_ulong,
    ) -> FPDF_BOOL;

    /// Determines if `document` represents a tagged PDF.
    ///
    /// For the definition of tagged PDF, see 10.7 "Tagged PDF" in PDF Reference 1.7.
    ///
    ///    `document` - handle to a document.
    ///
    /// Returns `true` if `document` is a tagged PDF.
    #[allow(non_snake_case)]
    fn FPDFCatalog_IsTagged(&self, document: FPDF_DOCUMENT) -> FPDF_BOOL;

    #[cfg(any(feature = "pdfium_6666", feature = "pdfium_future"))]
    #[doc = " Experimental API.\n Sets the language of |document| to |language|.\n\n document - handle to a document.\n language - the language to set to.\n\n Returns TRUE on success."]
    #[allow(non_snake_case)]
    fn FPDFCatalog_SetLanguage(&self, document: FPDF_DOCUMENT, language: &str) -> FPDF_BOOL;
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::utils::test::test_bind_to_pdfium;

    #[test]
    fn test_is_true() -> Result<(), PdfiumError> {
        let pdfium = test_bind_to_pdfium();

        assert!(!pdfium.bindings().is_true(0));
        assert!(pdfium.bindings().is_true(1));
        assert!(pdfium.bindings().is_true(-1));

        Ok(())
    }
}
