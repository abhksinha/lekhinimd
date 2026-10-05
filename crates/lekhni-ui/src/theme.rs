//! Theme palette and layout metrics.
//!
//! Follows Section 9 of LEKHNI_ARCHITECTURE:
//! Static struct of colors and metrics, swap = one pointer, mark all dirty.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub bg_workspace: u32,
    pub bg_sidebar: u32,
    pub bg_page_list: u32,
    pub bg_editor: u32,
    pub bg_preview: u32,
    pub border_color: u32,
    pub text_primary: u32,
    pub text_muted: u32,
    pub accent: u32,
    pub selection: u32,
    pub cursor_color: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

impl Theme {
    pub const fn dark() -> Self {
        Self {
            bg_workspace: 0xFF_18_18_18,
            bg_sidebar: 0xFF_1E_1E_1E,
            bg_page_list: 0xFF_25_25_26,
            bg_editor: 0xFF_1E_1E_1E,
            bg_preview: 0xFF_1E_1E_1E,
            border_color: 0xFF_33_33_33,
            text_primary: 0xFF_DC_DC_DC,
            text_muted: 0xFF_88_88_88,
            accent: 0xFF_00_7A_CC,
            selection: 0x66_26_4F_78,
            cursor_color: 0xFF_AE_AF_AD,
        }
    }

    pub const fn light() -> Self {
        Self {
            bg_workspace: 0xFF_F3_F3_F3,
            bg_sidebar: 0xFF_F8_F8_F8,
            bg_page_list: 0xFF_FF_FF_FF,
            bg_editor: 0xFF_FF_FF_FF,
            bg_preview: 0xFF_FF_FF_FF,
            border_color: 0xFF_E0_E0_E0,
            text_primary: 0xFF_22_22_22,
            text_muted: 0xFF_77_77_77,
            accent: 0xFF_00_66_B8,
            selection: 0x66_AD_D6_FF,
            cursor_color: 0xFF_00_00_00,
        }
    }
}
