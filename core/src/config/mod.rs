mod fastfetch;
mod icons;
mod presets;
#[cfg(test)]
mod tests;
mod validation;

pub use fastfetch::strip_jsonc;
pub use icons::{default_icon_for_module, distro_icon};
pub use validation::{
    BAR_STYLES, SHADING_MODES, canonical_shading_mode, is_known_bar_style, is_known_module_name,
    is_known_shading_mode, is_known_theme_name,
};
use validation::{collect_config_warnings, normalize_config_values, push_warning};

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct GpuConfig {
    #[serde(alias = "gpu_type", alias = "filter", alias = "display")]
    pub r#type: String,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            r#type: "dgpu".to_string(),
        }
    }
}

fn is_default_gpu_config(cfg: &GpuConfig) -> bool {
    cfg.r#type == "dgpu"
}

#[derive(Debug, Clone, Serialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub modules: Vec<ModuleConfig>,
    #[serde(skip_serializing_if = "is_default_gpu_config")]
    pub gpu: GpuConfig,
    /// Diagnostics collected while this configuration was loaded.
    ///
    /// Not part of the configuration identity: it is skipped during
    /// serialization and ignored by equality checks. Read it through
    /// [`Config::warnings`], which also re-validates the current values.
    #[serde(skip)]
    pub load_warnings: Vec<String>,
}

impl PartialEq for Config {
    fn eq(&self, other: &Self) -> bool {
        self.general == other.general && self.modules == other.modules && self.gpu == other.gpu
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum GpuConfigHelper {
    Table(GpuConfig),
    TypeStr(String),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ModulesFieldHelper {
    List(Vec<ModuleConfig>),
    Table(ModulesSectionHelper),
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ModulesSectionHelper {
    gpu: Option<GpuConfigHelper>,
    list: Option<Vec<ModuleConfig>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ConfigRaw {
    /// Optional named preset used as the base configuration, e.g. `preset = "card"`.
    preset: Option<String>,
    general: Option<GeneralConfigRaw>,
    modules: Option<ModulesFieldHelper>,
    gpu: Option<GpuConfigHelper>,
    border: Option<BorderFieldHelper>,
    border_image: Option<BorderImageFieldHelper>,
    border_images: Option<Vec<BorderImageConfig>>,
    colors: Option<ColorsConfig>,
    logo: Option<LogoConfig>,
    /// Unknown top-level keys are collected so [`Config::warnings`] can report them
    /// instead of silently ignoring a typo.
    #[serde(flatten)]
    unknown: BTreeMap<String, serde::de::IgnoredAny>,
}

impl<'de> Deserialize<'de> for Config {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = ConfigRaw::deserialize(deserializer)?;
        let mut warnings: Vec<String> = Vec::new();

        for key in raw.unknown.keys() {
            warnings.push(format!("unknown top-level key '{key}' ignored"));
        }

        // A named preset is the base layer; explicit user values are applied on top.
        let mut cfg = match raw.preset.as_deref().map(str::trim) {
            Some(name) if !name.is_empty() => match Config::from_preset(name) {
                Some(base) => base,
                None => {
                    warnings.push(format!(
                        "unknown preset '{name}' ignored (falling back to defaults)"
                    ));
                    Config::default()
                }
            },
            _ => Config::default(),
        };

        if let Some(general_raw) = raw.general {
            apply_partial_general_raw(&mut cfg.general, general_raw);
        }

        if let Some(b) = raw.border {
            match b {
                BorderFieldHelper::Bool(en) => cfg.general.border = en,
                BorderFieldHelper::Style(s) => {
                    cfg.general.border = true;
                    cfg.general.border_style = Some(s);
                }
                BorderFieldHelper::Table(tbl) => apply_border_helper(&mut cfg.general, *tbl),
            }
        }

        if let Some(bif) = raw.border_image {
            apply_border_image_helper(&mut cfg.general, bif);
        }

        if let Some(imgs) = raw.border_images {
            cfg.general.border_images = imgs;
        }
        if let Some(cols) = raw.colors {
            cfg.general.colors = cols;
        }
        if let Some(l) = raw.logo {
            cfg.general.logo = l;
        }

        let (mut modules, table_gpu) = match raw.modules {
            Some(ModulesFieldHelper::List(list)) => (list, None),
            Some(ModulesFieldHelper::Table(tbl)) => {
                (tbl.list.unwrap_or_else(|| cfg.modules.clone()), tbl.gpu)
            }
            None => (cfg.modules.clone(), None),
        };

        let raw_gpu = raw.gpu.or(table_gpu);
        cfg.gpu = match raw_gpu {
            Some(GpuConfigHelper::Table(gpu_cfg)) => gpu_cfg,
            Some(GpuConfigHelper::TypeStr(s)) => GpuConfig { r#type: s },
            None => GpuConfig::default(),
        };

        if cfg.gpu.r#type != "dgpu" {
            for m in &mut modules {
                if m.name == "gpu" && m.gpu_type.is_none() {
                    m.gpu_type = Some(cfg.gpu.r#type.clone());
                }
            }
        }
        cfg.modules = modules;

        normalize_config_values(&mut cfg, &mut warnings);
        cfg.load_warnings = warnings;
        Ok(cfg)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct ColorsConfig {
    pub enabled: bool,
    pub symbol: String,
    pub block: bool,
    pub image_palette: bool,
    pub position: String,
    pub rows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom: Option<Vec<String>>,
}

impl Default for ColorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            symbol: "●".to_string(),
            block: false,
            image_palette: true,
            position: "bottom".to_string(),
            rows: 1,
            custom: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BorderCharsConfig {
    pub top_left: String,
    pub top: String,
    pub top_right: String,
    pub right: String,
    pub bottom_right: String,
    pub bottom: String,
    pub bottom_left: String,
    pub left: String,
    pub divider_left: String,
    pub divider: String,
    pub divider_right: String,
}

/// Table form of `border_chars` (kept for backwards compatibility).
///
/// Missing fields fall back to the rounded style, matching the historical
/// table behaviour.
#[derive(Deserialize)]
#[serde(default)]
struct BorderCharsTable {
    top_left: String,
    top: String,
    top_right: String,
    right: String,
    bottom_right: String,
    bottom: String,
    bottom_left: String,
    left: String,
    divider_left: String,
    divider: String,
    divider_right: String,
}

impl Default for BorderCharsTable {
    fn default() -> Self {
        let base = BorderCharsConfig::default();
        Self {
            top_left: base.top_left,
            top: base.top,
            top_right: base.top_right,
            right: base.right,
            bottom_right: base.bottom_right,
            bottom: base.bottom,
            bottom_left: base.bottom_left,
            left: base.left,
            divider_left: base.divider_left,
            divider: base.divider,
            divider_right: base.divider_right,
        }
    }
}

impl<'de> Deserialize<'de> for BorderCharsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Helper {
            Table(Box<BorderCharsTable>),
            CompactString(String),
        }

        match Helper::deserialize(deserializer)? {
            Helper::Table(t) => Ok(BorderCharsConfig {
                top_left: t.top_left,
                top: t.top,
                top_right: t.top_right,
                right: t.right,
                bottom_right: t.bottom_right,
                bottom: t.bottom,
                bottom_left: t.bottom_left,
                left: t.left,
                divider_left: t.divider_left,
                divider: t.divider,
                divider_right: t.divider_right,
            }),
            Helper::CompactString(value) => {
                BorderCharsConfig::from_chars_string(&value).map_err(serde::de::Error::custom)
            }
        }
    }
}

impl Default for BorderCharsConfig {
    fn default() -> Self {
        Self::from_style("rounded")
    }
}

impl BorderCharsConfig {
    pub fn from_style(style: &str) -> Self {
        let (tl, t, tr, r, br, b, bl, l, dl, d, dr) = match style.to_lowercase().as_str() {
            "single" | "light" => ("┌", "─", "┐", "│", "┘", "─", "└", "│", "├", "─", "┤"),
            "double" => ("╔", "═", "╗", "║", "╝", "═", "╚", "║", "╠", "═", "╣"),
            "thick" | "heavy" => ("┏", "━", "┓", "┃", "┛", "━", "┗", "┃", "┣", "━", "┫"),
            "dashed" | "dots" => ("┌", "╌", "┐", "┆", "┘", "╌", "└", "┆", "├", "╌", "┤"),
            "brackets" => ("[", "─", "]", "│", "]", "─", "[", "│", "[", "─", "]"),
            "ascii" => ("+", "-", "+", "|", "+", "-", "+", "|", "+", "-", "+"),
            "block" => ("█", "▀", "█", "█", "█", "▄", "█", "█", "█", "▀", "█"),
            _ => ("╭", "─", "╮", "│", "╯", "─", "╰", "│", "├", "─", "┤"),
        };
        Self {
            top_left: tl.to_string(),
            top: t.to_string(),
            top_right: tr.to_string(),
            right: r.to_string(),
            bottom_right: br.to_string(),
            bottom: b.to_string(),
            bottom_left: bl.to_string(),
            left: l.to_string(),
            divider_left: dl.to_string(),
            divider: d.to_string(),
            divider_right: dr.to_string(),
        }
    }

    /// Builds border characters from an 11-character compact string such as
    /// `"┌─┐│┘─└│├─┤"` (top-left, top, top-right, right, bottom-right,
    /// bottom, bottom-left, left, divider-left, divider, divider-right).
    pub fn from_chars_string(value: &str) -> Result<Self, String> {
        let chars: Vec<char> = value.trim().chars().collect();
        if chars.len() != 11 {
            return Err(format!(
                "border_chars compact string must contain exactly 11 characters (got {})",
                chars.len()
            ));
        }
        let s = |index: usize| chars[index].to_string();
        Ok(Self {
            top_left: s(0),
            top: s(1),
            top_right: s(2),
            right: s(3),
            bottom_right: s(4),
            bottom: s(5),
            bottom_left: s(6),
            left: s(7),
            divider_left: s(8),
            divider: s(9),
            divider_right: s(10),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct ThreeDConfig {
    pub enabled: bool,
    pub speed: f32,
    pub rotate_x: bool,
    pub rotate_y: bool,
    pub size: f32,
    pub depth: f32,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "image_width_cols",
        alias = "cols"
    )]
    pub width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "rows")]
    pub height: Option<usize>,
    pub shading_mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shading: Option<String>,
    pub light: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frames: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub infinite: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_on_key: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outer_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inner_color: Option<String>,
}

impl Default for ThreeDConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            speed: 1.0,
            rotate_x: true,
            rotate_y: true,
            size: 1.0,
            depth: 1.0,
            width: Some(42),
            height: Some(24),
            shading_mode: "ascii".to_string(),
            shading: None,
            light: "top-left".to_string(),
            frames: None,
            infinite: None,
            hold: None,
            exit_on_key: Some(true),
            outer_color: None,
            inner_color: None,
        }
    }
}

fn deserialize_three_d<'de, D>(deserializer: D) -> Result<ThreeDConfig, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Helper {
        Bool(bool),
        Config(ThreeDConfig),
    }

    match Helper::deserialize(deserializer)? {
        Helper::Bool(b) => Ok(ThreeDConfig {
            enabled: b,
            ..ThreeDConfig::default()
        }),
        Helper::Config(cfg) => Ok(cfg),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct GeneralConfig {
    pub separator: String,
    pub padding: usize,
    pub center: bool,
    pub icons: bool,
    pub icon_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,
    pub live: bool,
    pub check_updates: bool,
    #[serde(skip_serializing_if = "Option::is_none", alias = "clear")]
    pub clear_screen: Option<bool>,
    pub border: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_title: Option<String>,
    pub box_padding: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_image_width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_image_height: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_image_offset_x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_image_offset_y: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_image_z_index: Option<i32>,
    pub border_image_position: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_image_protocol: Option<String>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "border_image_preserveAspectRatio"
    )]
    pub border_image_preserve_aspect_ratio: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        alias = "border_image_textOnTop",
        alias = "border_image_text_over"
    )]
    pub border_image_text_on_top: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(alias = "3d", default, skip_serializing_if = "Option::is_none")]
    pub three_d: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub border_chars: Option<BorderCharsConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub border_images: Vec<BorderImageConfig>,
    pub colors: ColorsConfig,
    pub logo: LogoConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct BorderImageConfig {
    pub path: String,
    pub position: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_x: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_y: Option<i32>,
    pub z_index: i32,
    pub protocol: String,
    #[serde(default = "default_true", alias = "preserveAspectRatio")]
    pub preserve_aspect_ratio: bool,
    #[serde(
        default = "default_true",
        alias = "text_over",
        alias = "textOnTop",
        alias = "textOver"
    )]
    pub text_on_top: bool,
}

fn default_true() -> bool {
    true
}

impl Default for BorderImageConfig {
    fn default() -> Self {
        Self {
            path: String::new(),
            position: "behind".to_string(),
            width: None,
            height: None,
            offset_x: None,
            offset_y: None,
            z_index: -1,
            protocol: "auto".to_string(),
            preserve_aspect_ratio: true,
            text_on_top: true,
        }
    }
}

impl BorderImageConfig {
    pub fn is_background(&self) -> bool {
        let pos = self.position.trim().to_ascii_lowercase();
        pos == "background" || pos == "behind" || pos == "bg"
    }

    pub fn effective_z_index(&self) -> i32 {
        if self.text_on_top {
            -1
        } else if self.z_index <= 0 {
            1
        } else {
            self.z_index
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct BorderSectionHelper {
    #[serde(alias = "border")]
    pub enabled: Option<bool>,
    #[serde(alias = "border_style")]
    pub style: Option<String>,
    #[serde(alias = "border_color")]
    pub color: Option<String>,
    #[serde(alias = "border_title")]
    pub title: Option<String>,
    #[serde(alias = "box_padding")]
    pub padding: Option<usize>,
    #[serde(alias = "border_chars")]
    pub chars: Option<BorderCharsConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum BorderFieldHelper {
    Bool(bool),
    Style(String),
    Table(Box<BorderSectionHelper>),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Default)]
#[serde(default)]
pub struct BorderImageSectionHelper {
    #[serde(alias = "border_image")]
    pub path: Option<String>,
    #[serde(alias = "border_image_position")]
    pub position: Option<String>,
    #[serde(alias = "border_image_width")]
    pub width: Option<usize>,
    #[serde(alias = "border_image_height")]
    pub height: Option<usize>,
    #[serde(alias = "border_image_offset_x")]
    pub offset_x: Option<i32>,
    #[serde(alias = "border_image_offset_y")]
    pub offset_y: Option<i32>,
    #[serde(alias = "border_image_z_index")]
    pub z_index: Option<i32>,
    #[serde(alias = "border_image_protocol")]
    pub protocol: Option<String>,
    #[serde(
        alias = "border_image_preserve_aspect_ratio",
        alias = "preserve_aspect_ratio",
        alias = "preserveAspectRatio"
    )]
    pub preserve_aspect_ratio: Option<bool>,
    #[serde(
        alias = "border_image_text_on_top",
        alias = "text_on_top",
        alias = "text_over",
        alias = "textOnTop",
        alias = "textOver"
    )]
    pub text_on_top: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(untagged)]
pub enum BorderImageFieldHelper {
    Path(String),
    Table(BorderImageSectionHelper),
}

pub fn apply_border_helper(cfg: &mut GeneralConfig, b: BorderSectionHelper) {
    if let Some(en) = b.enabled {
        cfg.border = en;
    }
    if let Some(s) = b.style {
        cfg.border_style = Some(s);
    }
    if let Some(c) = b.color {
        cfg.border_color = Some(c);
    }
    if let Some(t) = b.title {
        cfg.border_title = Some(t);
    }
    if let Some(p) = b.padding {
        cfg.box_padding = p;
    }
    if let Some(ch) = b.chars {
        cfg.border_chars = Some(ch);
    }
}

pub fn apply_border_image_helper(cfg: &mut GeneralConfig, bif: BorderImageFieldHelper) {
    match bif {
        BorderImageFieldHelper::Path(p) => {
            if !p.trim().is_empty() {
                cfg.border_image = Some(p);
            }
        }
        BorderImageFieldHelper::Table(tbl) => {
            if let Some(p) = tbl.path
                && !p.trim().is_empty()
            {
                cfg.border_image = Some(p);
            }
            if let Some(pos) = tbl.position {
                cfg.border_image_position = pos;
            }
            if let Some(w) = tbl.width {
                cfg.border_image_width = Some(w);
            }
            if let Some(h) = tbl.height {
                cfg.border_image_height = Some(h);
            }
            if let Some(ox) = tbl.offset_x {
                cfg.border_image_offset_x = Some(ox);
            }
            if let Some(oy) = tbl.offset_y {
                cfg.border_image_offset_y = Some(oy);
            }
            if let Some(zi) = tbl.z_index {
                cfg.border_image_z_index = Some(zi);
            }
            if let Some(pr) = tbl.protocol {
                cfg.border_image_protocol = Some(pr);
            }
            if let Some(par) = tbl.preserve_aspect_ratio {
                cfg.border_image_preserve_aspect_ratio = Some(par);
            }
            if let Some(tot) = tbl.text_on_top {
                cfg.border_image_text_on_top = Some(tot);
            }
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct GeneralConfigRaw {
    separator: Option<String>,
    padding: Option<usize>,
    center: Option<bool>,
    icons: Option<bool>,
    icon_only: Option<bool>,
    key_type: Option<String>,
    live: Option<bool>,
    #[serde(alias = "update_check", alias = "notify_updates")]
    check_updates: Option<bool>,
    #[serde(alias = "clear")]
    clear_screen: Option<bool>,

    border: Option<BorderFieldHelper>,
    border_style: Option<String>,
    border_color: Option<String>,
    border_chars: Option<BorderCharsConfig>,
    border_title: Option<String>,
    box_padding: Option<usize>,

    border_image: Option<BorderImageFieldHelper>,
    border_image_width: Option<usize>,
    border_image_height: Option<usize>,
    border_image_offset_x: Option<i32>,
    border_image_offset_y: Option<i32>,
    border_image_z_index: Option<i32>,
    border_image_position: Option<String>,
    border_image_protocol: Option<String>,
    #[serde(
        alias = "border_image_preserveAspectRatio",
        alias = "border_image_preserve_aspect_ratio"
    )]
    border_image_preserve_aspect_ratio: Option<bool>,
    #[serde(
        alias = "border_image_textOnTop",
        alias = "border_image_text_on_top",
        alias = "border_image_text_over",
        alias = "border_image_textOver"
    )]
    border_image_text_on_top: Option<bool>,

    border_images: Option<Vec<BorderImageConfig>>,
    gradient: Option<Vec<String>>,
    theme: Option<String>,
    colors: Option<ColorsConfig>,
    logo: Option<LogoConfig>,
    #[serde(alias = "3d")]
    three_d: Option<bool>,
}

fn apply_partial_general_raw(cfg: &mut GeneralConfig, raw: GeneralConfigRaw) {
    if let Some(v) = raw.separator {
        cfg.separator = v;
    }
    if let Some(v) = raw.padding {
        cfg.padding = v;
    }
    if let Some(v) = raw.center {
        cfg.center = v;
    }
    if let Some(v) = raw.icons {
        cfg.icons = v;
    }
    if let Some(v) = raw.icon_only {
        cfg.icon_only = v;
    }
    if let Some(v) = raw.key_type {
        cfg.key_type = Some(v);
    }
    if let Some(v) = raw.live {
        cfg.live = v;
    }
    if let Some(cu) = raw.check_updates {
        cfg.check_updates = cu;
    }
    if let Some(cs) = raw.clear_screen {
        cfg.clear_screen = Some(cs);
    }

    if let Some(bf) = raw.border {
        match bf {
            BorderFieldHelper::Bool(b) => {
                cfg.border = b;
            }
            BorderFieldHelper::Style(s) => {
                cfg.border = true;
                cfg.border_style = Some(s);
            }
            BorderFieldHelper::Table(tbl) => {
                apply_border_helper(cfg, *tbl);
            }
        }
    }
    if let Some(s) = raw.border_style {
        cfg.border_style = Some(s);
    }
    if let Some(c) = raw.border_color {
        cfg.border_color = Some(c);
    }
    if let Some(ch) = raw.border_chars {
        cfg.border_chars = Some(ch);
    }
    if let Some(t) = raw.border_title {
        cfg.border_title = Some(t);
    }
    if let Some(p) = raw.box_padding {
        cfg.box_padding = p;
    }

    if let Some(bif) = raw.border_image {
        apply_border_image_helper(cfg, bif);
    }
    if let Some(pos) = raw.border_image_position {
        cfg.border_image_position = pos;
    }
    if let Some(w) = raw.border_image_width {
        cfg.border_image_width = Some(w);
    }
    if let Some(h) = raw.border_image_height {
        cfg.border_image_height = Some(h);
    }
    if let Some(ox) = raw.border_image_offset_x {
        cfg.border_image_offset_x = Some(ox);
    }
    if let Some(oy) = raw.border_image_offset_y {
        cfg.border_image_offset_y = Some(oy);
    }
    if let Some(zi) = raw.border_image_z_index {
        cfg.border_image_z_index = Some(zi);
    }
    if let Some(pr) = raw.border_image_protocol {
        cfg.border_image_protocol = Some(pr);
    }
    if let Some(par) = raw.border_image_preserve_aspect_ratio {
        cfg.border_image_preserve_aspect_ratio = Some(par);
    }
    if let Some(tot) = raw.border_image_text_on_top {
        cfg.border_image_text_on_top = Some(tot);
    }

    if let Some(images) = raw.border_images {
        cfg.border_images = images;
    }
    if let Some(g) = raw.gradient {
        cfg.gradient = Some(g);
    }
    if let Some(t) = raw.theme {
        cfg.theme = Some(t);
    }
    if let Some(c) = raw.colors {
        cfg.colors = c;
    }
    if let Some(l) = raw.logo {
        cfg.logo = l;
    }
    if let Some(td) = raw.three_d {
        cfg.three_d = Some(td);
    }
}

impl<'de> Deserialize<'de> for GeneralConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = GeneralConfigRaw::deserialize(deserializer)?;
        let mut cfg = GeneralConfig::default();
        apply_partial_general_raw(&mut cfg, raw);
        Ok(cfg)
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct AnimationConfig {
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub fps: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_on_key: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frames: Option<usize>,
    pub infinite: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hold: Option<bool>,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            path: None,
            fps: 15.0,
            exit_on_key: Some(true),
            frames: None,
            infinite: true,
            hold: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct LogoConfig {
    pub enabled: bool,
    pub distro: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "ascii")]
    pub ascii_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub colors: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_path: Option<String>,
    pub image_width_cols: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_dir: Option<String>,
    pub random_image: bool,
    pub auto_color: bool,
    pub protocol: String,
    #[serde(alias = "3d", default, deserialize_with = "deserialize_three_d")]
    pub three_d: ThreeDConfig,
    #[serde(alias = "anim", default)]
    pub animation: AnimationConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ModuleConfig {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bar: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bar_width: Option<usize>,
    /// Progress-bar glyph style: "blocks" (default), "line", "dots", "ascii".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bar_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gpu_type: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ModuleConfigTableHelper {
    name: Option<String>,
    #[serde(rename = "type")]
    type_field: Option<String>,
    #[serde(alias = "filter", alias = "display")]
    gpu_type: Option<String>,
    gpu: Option<GpuConfigHelper>,
    module: Option<String>,
    id: Option<String>,
    #[serde(alias = "text", alias = "title", alias = "key")]
    label: Option<String>,
    #[serde(alias = "logo", alias = "symbol", alias = "prefix")]
    icon: Option<String>,
    color: Option<String>,
    value: Option<String>,
    format: Option<String>,
    bar: Option<bool>,
    bar_width: Option<usize>,
    #[serde(alias = "bar-style")]
    bar_style: Option<String>,
    #[serde(alias = "mount", alias = "mount_point")]
    path: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum ModuleConfigHelper {
    Name(String),
    Table(Box<ModuleConfigTableHelper>),
}

impl<'de> Deserialize<'de> for ModuleConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match ModuleConfigHelper::deserialize(deserializer)? {
            ModuleConfigHelper::Name(name) => Ok(ModuleConfig::new(name)),
            ModuleConfigHelper::Table(t) => {
                let (name, mut gpu_type) = match (t.name, t.module, t.id, t.type_field) {
                    (Some(n), _, _, tf) => {
                        if n == "gpu" {
                            (n, tf)
                        } else {
                            (n, None)
                        }
                    }
                    (None, Some(m), _, tf) => {
                        if m == "gpu" {
                            (m, tf)
                        } else {
                            (m, None)
                        }
                    }
                    (None, None, Some(id), tf) => {
                        if id == "gpu" {
                            (id, tf)
                        } else {
                            (id, None)
                        }
                    }
                    (None, None, None, Some(tf)) => (tf, None),
                    (None, None, None, None) => (String::new(), None),
                };

                if name == "gpu" {
                    if let Some(gt) = t.gpu_type {
                        gpu_type = Some(gt);
                    } else if let Some(gpu_helper) = t.gpu {
                        match gpu_helper {
                            GpuConfigHelper::Table(cfg) => gpu_type = Some(cfg.r#type),
                            GpuConfigHelper::TypeStr(s) => gpu_type = Some(s),
                        }
                    }
                }

                Ok(ModuleConfig {
                    name,
                    label: t.label,
                    icon: t.icon,
                    color: t.color,
                    value: t.value,
                    format: t.format,
                    bar: t.bar,
                    bar_width: t.bar_width,
                    bar_style: t.bar_style,
                    path: t.path,
                    gpu_type,
                })
            }
        }
    }
}

impl Default for LogoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            distro: "auto".to_string(),
            position: None,
            ascii_path: None,
            colors: None,
            image_path: None,
            image_width_cols: 42,
            image_dir: None,
            random_image: false,
            auto_color: true,
            protocol: "auto".to_string(),
            three_d: ThreeDConfig::default(),
            animation: AnimationConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            separator: ":".to_string(),
            padding: 1,
            center: false,
            icons: true,
            icon_only: false,
            key_type: None,
            live: false,
            check_updates: true,
            clear_screen: None,
            border: false,
            border_style: None,
            border_color: None,
            border_chars: None,
            border_title: None,
            box_padding: 1,
            border_image: None,
            border_image_width: None,
            border_image_height: None,
            border_image_offset_x: None,
            border_image_offset_y: None,
            border_image_z_index: None,
            border_image_position: "behind".to_string(),
            border_image_protocol: None,
            border_image_preserve_aspect_ratio: None,
            border_image_text_on_top: None,
            border_images: Vec::new(),
            gradient: None,
            theme: None,
            colors: ColorsConfig::default(),
            logo: LogoConfig::default(),
            three_d: None,
        }
    }
}

impl GeneralConfig {
    pub fn active_border_images(&self) -> Vec<BorderImageConfig> {
        let mut list = Vec::new();
        if let Some(path) = &self.border_image
            && !path.trim().is_empty()
        {
            list.push(BorderImageConfig {
                path: path.clone(),
                position: self.border_image_position.clone(),
                width: self.border_image_width,
                height: self.border_image_height,
                offset_x: self.border_image_offset_x,
                offset_y: self.border_image_offset_y,
                z_index: self.border_image_z_index.unwrap_or(-1),
                protocol: self
                    .border_image_protocol
                    .clone()
                    .unwrap_or_else(|| "auto".to_string()),
                preserve_aspect_ratio: self.border_image_preserve_aspect_ratio.unwrap_or(true),
                text_on_top: self.border_image_text_on_top.unwrap_or(true),
            });
        }
        for img in &self.border_images {
            if !img.path.trim().is_empty() {
                list.push(img.clone());
            }
        }
        list
    }
}

impl ModuleConfig {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Self::default()
        }
    }
}

pub(super) fn modules_from(names: &[&str]) -> Vec<ModuleConfig> {
    names.iter().copied().map(ModuleConfig::new).collect()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            modules: modules_from(&[
                "os", "host", "board", "kernel", "uptime", "packages", "shell", "desktop", "font",
                "terminal", "cpu", "gpu", "memory", "swap", "disk", "battery", "local_ip",
                "locale",
            ]),
            gpu: GpuConfig::default(),
            load_warnings: Vec::new(),
        }
    }
}

impl Config {
    /// Base configuration directory: `$XDG_CONFIG_HOME` when set to a non-empty
    /// value, otherwise `$HOME/.config`. No literal `~` is ever produced; when
    /// neither variable is usable a relative `.config` directory is returned so
    /// callers can still report a printable path.
    pub fn config_dir() -> PathBuf {
        Self::config_dir_from_env(|var| std::env::var(var).ok())
    }

    fn config_dir_from_env(get_env: impl Fn(&str) -> Option<String>) -> PathBuf {
        if let Some(xdg) = get_env("XDG_CONFIG_HOME") {
            let trimmed = xdg.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }
        if let Some(home) = get_env("HOME") {
            let trimmed = home.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed).join(".config");
            }
        }
        PathBuf::from(".config")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("rustfetch").join("config.toml")
    }

    /// Diagnostics about this configuration: ignored/unknown keys and module
    /// names, unknown presets/themes/shading modes/bar styles, and values that
    /// were clamped while loading.
    ///
    /// An empty vector means the configuration is fully valid. This is the API
    /// exposed to `rustfetch --doctor`; messages are plain text.
    pub fn warnings(&self) -> Vec<String> {
        let mut warnings: Vec<String> = Vec::new();
        for message in &self.load_warnings {
            push_warning(&mut warnings, message.clone());
        }
        collect_config_warnings(self, &mut warnings);
        warnings
    }

    /// Applies the centralized value normalization/bounds to a programmatically
    /// built configuration (the load paths already do this automatically).
    pub fn clamp(&mut self) {
        let mut warnings = Vec::new();
        normalize_config_values(self, &mut warnings);
        for message in warnings {
            push_warning(&mut self.load_warnings, message);
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            let imported = Self::find_fastfetch_config()
                .and_then(|ff_path| std::fs::read_to_string(ff_path).ok())
                .and_then(|content| Self::import_from_fastfetch(&content));

            let (cfg, toml_out) = match imported {
                Some(imp) => {
                    let s =
                        toml::to_string_pretty(&imp).unwrap_or_else(|_| Self::to_default_toml());
                    (imp, s)
                }
                None => (Self::default(), Self::to_default_toml()),
            };

            Self::write_if_not_exists(&path, &toml_out);
            return cfg;
        }
        Self::load_from(&path).unwrap_or_else(|err| {
            eprintln!("{err}");
            Self::default()
        })
    }

    /// Writes `content` to `path` ONLY if `path` does not already exist (`create_new(true)`).
    /// Guarantees that an existing user configuration file is never overwritten.
    pub fn write_if_not_exists(path: &Path, content: &str) -> bool {
        if path.exists() {
            return false;
        }
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            file.write_all(content.as_bytes()).is_ok()
        } else {
            false
        }
    }

    /// Creates a `.bak` backup of `path` if `path` exists and is non-empty.
    pub fn backup_if_exists(path: &Path) -> Option<PathBuf> {
        if !path.is_file() {
            return None;
        }
        let meta = std::fs::metadata(path).ok()?;
        if meta.len() == 0 {
            return None;
        }
        let mut bak = path.as_os_str().to_os_string();
        bak.push(".bak");
        let bak_path = PathBuf::from(bak);
        std::fs::copy(path, &bak_path).ok()?;
        Some(bak_path)
    }

    pub fn load_from(path: &Path) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path.display(), e))?;
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext == "json" || ext == "jsonc")
            && let Some(cfg) = Self::import_from_fastfetch(&content)
        {
            return Ok(cfg);
        }
        Self::from_toml(&content).or_else(|toml_err| {
            if let Some(cfg) = Self::import_from_fastfetch(&content) {
                Ok(cfg)
            } else {
                Err(format!(
                    "Failed to parse config file '{}': {toml_err}",
                    path.display()
                ))
            }
        })
    }

    pub fn from_toml(content: &str) -> Result<Self, toml::de::Error> {
        match toml::from_str::<Self>(content) {
            Ok(cfg) => Ok(cfg),
            Err(err) => {
                if let Some(salvaged) = Self::salvage_partial_toml(content) {
                    Ok(salvaged)
                } else {
                    Err(err)
                }
            }
        }
    }

    fn salvage_modules_array(mods_arr: &[toml::Value], out: &mut Vec<ModuleConfig>) -> bool {
        let mut valid_mods = Vec::new();
        for item in mods_arr {
            if let Ok(mc) = ModuleConfig::deserialize(item.clone())
                && !mc.name.trim().is_empty()
            {
                valid_mods.push(mc);
            }
        }
        if !valid_mods.is_empty() {
            *out = valid_mods;
            true
        } else {
            false
        }
    }

    /// Fault-tolerant recovery for older or partially invalid user `config.toml` files.
    /// If a single field has a deprecated or invalid value type, preserves all other valid
    /// user settings instead of discarding the entire configuration.
    fn salvage_partial_toml(content: &str) -> Option<Self> {
        let root: toml::Table = content.parse().ok()?;
        let mut cfg = Config::default();
        let mut recovered_any = false;

        if let Some(preset_cfg) = root
            .get("preset")
            .and_then(|v| v.as_str())
            .and_then(Config::from_preset)
        {
            cfg = preset_cfg;
            recovered_any = true;
        }

        if let Some(toml::Value::Table(gen_tbl)) = root.get("general") {
            for (k, v) in gen_tbl {
                let mut single_gen = toml::Table::new();
                single_gen.insert(k.clone(), v.clone());
                let val = toml::Value::Table(single_gen);
                if let Ok(partial_raw) = GeneralConfigRaw::deserialize(val) {
                    apply_partial_general_raw(&mut cfg.general, partial_raw);
                    recovered_any = true;
                }
            }
        }

        for top_key in [
            "border",
            "border_image",
            "border_images",
            "colors",
            "logo",
            "gpu",
        ] {
            if let Some(v) = root.get(top_key) {
                let mut single_top = toml::Table::new();
                single_top.insert(top_key.to_string(), v.clone());
                if let Ok(partial_cfg) = Config::deserialize(toml::Value::Table(single_top)) {
                    match top_key {
                        "border" => {
                            cfg.general.border = partial_cfg.general.border;
                            cfg.general.border_style = partial_cfg.general.border_style;
                            cfg.general.border_color = partial_cfg.general.border_color;
                            cfg.general.border_chars = partial_cfg.general.border_chars;
                            cfg.general.border_title = partial_cfg.general.border_title;
                            cfg.general.box_padding = partial_cfg.general.box_padding;
                        }
                        "border_image" => {
                            cfg.general.border_image = partial_cfg.general.border_image;
                            cfg.general.border_image_position =
                                partial_cfg.general.border_image_position;
                            cfg.general.border_image_width = partial_cfg.general.border_image_width;
                            cfg.general.border_image_height =
                                partial_cfg.general.border_image_height;
                            cfg.general.border_image_offset_x =
                                partial_cfg.general.border_image_offset_x;
                            cfg.general.border_image_offset_y =
                                partial_cfg.general.border_image_offset_y;
                            cfg.general.border_image_z_index =
                                partial_cfg.general.border_image_z_index;
                            cfg.general.border_image_protocol =
                                partial_cfg.general.border_image_protocol;
                            cfg.general.border_image_preserve_aspect_ratio =
                                partial_cfg.general.border_image_preserve_aspect_ratio;
                            cfg.general.border_image_text_on_top =
                                partial_cfg.general.border_image_text_on_top;
                        }
                        "border_images" => {
                            cfg.general.border_images = partial_cfg.general.border_images;
                        }
                        "colors" => {
                            cfg.general.colors = partial_cfg.general.colors;
                        }
                        "logo" => {
                            cfg.general.logo = partial_cfg.general.logo;
                        }
                        "gpu" => {
                            cfg.gpu = partial_cfg.gpu;
                        }
                        _ => {}
                    }
                    recovered_any = true;
                }
            }
        }

        if let Some(toml::Value::Table(mods_tbl)) = root.get("modules") {
            if let Some(gpu_val) = mods_tbl.get("gpu") {
                let mut single_gpu = toml::Table::new();
                single_gpu.insert("gpu".to_string(), gpu_val.clone());
                if let Ok(partial_cfg) = Config::deserialize(toml::Value::Table(single_gpu)) {
                    cfg.gpu = partial_cfg.gpu;
                    recovered_any = true;
                }
            }
            if let Some(toml::Value::Array(arr)) = mods_tbl.get("list")
                && Self::salvage_modules_array(arr, &mut cfg.modules)
            {
                recovered_any = true;
            }
        }

        if let Some(toml::Value::Array(mods_arr)) = root.get("modules")
            && Self::salvage_modules_array(mods_arr, &mut cfg.modules)
        {
            recovered_any = true;
        }

        if cfg.gpu.r#type != "dgpu" {
            for m in &mut cfg.modules {
                if m.name == "gpu" && m.gpu_type.is_none() {
                    m.gpu_type = Some(cfg.gpu.r#type.clone());
                }
            }
        }

        if recovered_any {
            let mut warnings = Vec::new();
            normalize_config_values(&mut cfg, &mut warnings);
            cfg.load_warnings = warnings;
            Some(cfg)
        } else {
            None
        }
    }

    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }
}
