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
    pub border_chars: Option<BorderCharsConfig>,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub border_images: Vec<BorderImageConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gradient: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    pub colors: ColorsConfig,
    pub logo: LogoConfig,
    #[serde(alias = "3d", default)]
    pub three_d: Option<bool>,
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
            if let Some(p) = tbl.path {
                if !p.trim().is_empty() {
                    cfg.border_image = Some(p);
                }
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
    pub image_path: Option<String>,
    pub image_width_cols: usize,
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
        if let Some(path) = &self.border_image {
            if !path.trim().is_empty() {
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

fn modules_from(names: &[&str]) -> Vec<ModuleConfig> {
    names.iter().copied().map(ModuleConfig::new).collect()
}

pub fn default_icon_for_module(name: &str) -> Option<&'static str> {
    if name.starts_with("disk:") {
        return Some("󰋊");
    }
    match name {
        "os" => Some("󰌽"),
        "os_age" | "age" => Some("󰃭"),
        "host" => Some("󰌢"),
        "board" => Some("󰘚"),
        "kernel" => Some("󰒋"),
        "uptime" => Some("󱑂"),
        "packages" => Some("󰏖"),
        "shell" => Some("󰞷"),
        "desktop" => Some("󰨇"),
        "font" => Some("󰛖"),
        "terminal" => Some("󰆍"),
        "cpu" => Some("󰍛"),
        "gpu" | "dgpu" | "igpu" => Some("󰢮"),
        "gpu_driver" | "driver" => Some("󰢮"),
        "memory" => Some("󰘚"),
        "swap" => Some("󰓡"),
        "disk" => Some("󰋊"),
        "battery" => Some("󰂁"),
        "power_profile" | "power" => Some("󰓅"),
        "brightness" => Some("󰃠"),
        "cpu_usage" => Some("󰓅"),
        "bluetooth" => Some("󰂯"),
        "local_ip" => Some("󰩟"),
        "net_io" | "network" | "net" => Some("󰛳"),
        "locale" => Some("󰗊"),
        "temp" => Some("󰔏"),
        "sound" => Some("󰕾"),
        "media" => Some("󰝚"),
        "display" => Some("󰍹"),
        "wifi" => Some("󰖩"),
        "processes" => Some("󰒋"),
        "init" | "initsystem" => Some(""),
        "container" | "vm" => Some("󰡨"),
        "update" => Some("󰚰"),
        "custom" => Some("󰅐"),
        _ => None,
    }
}

pub fn distro_icon(distro_name: &str) -> &'static str {
    let lower = distro_name.to_lowercase();
    if lower.contains("arch") {
        "󰣇"
    } else if lower.contains("ubuntu") {
        "󰕈"
    } else if lower.contains("fedora") || lower.contains("bazzite") || lower.contains("nobara") {
        "󰣛"
    } else if lower.contains("debian") {
        "󰣚"
    } else if lower.contains("nixos") {
        "󱄅"
    } else if lower.contains("gentoo") {
        "󰣨"
    } else if lower.contains("manjaro") {
        "󱘊"
    } else if lower.contains("mint") {
        "󰣭"
    } else if lower.contains("alpine") {
        "󰣠"
    } else if lower.contains("suse") {
        "󰣡"
    } else if lower.contains("kali") {
        "󰣳"
    } else if lower.contains("pop") {
        "󰣩"
    } else if lower.contains("endeavour") {
        "󰣫"
    } else if lower.contains("artix") {
        "󰣢"
    } else if lower.contains("void") {
        "󰣲"
    } else if lower.contains("cachy") {
        "󰣇"
    } else if lower.contains("darwin") || lower.contains("macos") || lower.contains("apple") {
        "󰀵"
    } else if lower.contains("windows") {
        "󰍲"
    } else if lower.contains("android") {
        "󰀲"
    } else if lower.contains("freebsd") || lower.contains("bsd") {
        "󰣶"
    } else {
        "󰌽"
    }
}

/// Shading modes understood by the 3D renderer.
pub const SHADING_MODES: &[&str] = &[
    "ascii",
    "blocks",
    "sextants",
    "braille",
    "dot",
    "wireframe",
    "solid",
];

/// Progress-bar glyph styles understood by the layout engine.
pub const BAR_STYLES: &[&str] = &["blocks", "line", "dots", "ascii"];

/// Theme identifiers accepted by `rustfetch_render::get_theme`.
///
/// Kept here as a dependency-free copy (core must not depend on the render
/// crate); `render/src/color.rs` is the authority and this list must be kept
/// in sync with its canonical names and aliases.
const KNOWN_THEMES: &[&str] = &[
    "catppuccin-mocha",
    "catppuccin",
    "mocha",
    "catppuccin-latte",
    "latte",
    "catppuccin-macchiato",
    "macchiato",
    "catppuccin-frappe",
    "frappe",
    "dracula",
    "tokyo-night",
    "tokyonight",
    "gruvbox",
    "gruvbox-dark",
    "nord",
    "rose-pine",
    "rosepine",
    "cyberpunk",
    "monokai",
    "onedark",
    "one-dark",
    "solarized-dark",
    "solarized",
];

/// Returns true when `name` is a module identifier accepted by [`crate::modules::module_by_name`]
/// (including the `disk:<mount>` form).
pub fn is_known_module_name(name: &str) -> bool {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return false;
    }
    if let Some(mount) = trimmed.strip_prefix("disk:") {
        return !mount.trim().is_empty();
    }
    crate::modules::module_names().contains(&trimmed)
}

/// Returns true when `name` is a supported 3D shading mode (case-insensitive).
pub fn is_known_shading_mode(name: &str) -> bool {
    SHADING_MODES.contains(&name.trim().to_lowercase().as_str())
}

/// Returns true when `name` is a supported progress-bar style (case-insensitive).
pub fn is_known_bar_style(name: &str) -> bool {
    BAR_STYLES.contains(&name.trim().to_lowercase().as_str())
}

/// Returns true when `name` resolves to a theme in the render crate.
pub fn is_known_theme_name(name: &str) -> bool {
    let normalized = name.trim().to_lowercase().replace('_', "-");
    KNOWN_THEMES.contains(&normalized.as_str())
}

fn push_warning(warnings: &mut Vec<String>, message: String) {
    if !warnings.contains(&message) {
        warnings.push(message);
    }
}

fn clamp_usize(value: &mut usize, min: usize, max: usize, path: &str, warnings: &mut Vec<String>) {
    if *value < min {
        push_warning(
            warnings,
            format!("{path} {value} is below the supported minimum {min}; clamped to {min}"),
        );
        *value = min;
    } else if *value > max {
        push_warning(
            warnings,
            format!("{path} {value} exceeds the supported maximum {max}; clamped to {max}"),
        );
        *value = max;
    }
}

fn clamp_optional_usize(
    value: &mut Option<usize>,
    min: usize,
    max: usize,
    path: &str,
    warnings: &mut Vec<String>,
) {
    if let Some(current) = value {
        if *current < min {
            push_warning(
                warnings,
                format!("{path} {current} is below the supported minimum {min}; clamped to {min}"),
            );
            *value = Some(min);
        } else if *current > max {
            push_warning(
                warnings,
                format!("{path} {current} exceeds the supported maximum {max}; clamped to {max}"),
            );
            *value = Some(max);
        }
    }
}

fn clamp_i32(value: &mut i32, min: i32, max: i32, path: &str, warnings: &mut Vec<String>) {
    if *value < min {
        push_warning(
            warnings,
            format!("{path} {value} is below the supported minimum {min}; clamped to {min}"),
        );
        *value = min;
    } else if *value > max {
        push_warning(
            warnings,
            format!("{path} {value} exceeds the supported maximum {max}; clamped to {max}"),
        );
        *value = max;
    }
}

fn clamp_optional_i32(
    value: &mut Option<i32>,
    min: i32,
    max: i32,
    path: &str,
    warnings: &mut Vec<String>,
) {
    if let Some(mut current) = *value {
        clamp_i32(&mut current, min, max, path, warnings);
        *value = Some(current);
    }
}

fn clamp_f32(
    value: &mut f32,
    min: f32,
    max: f32,
    default: f32,
    path: &str,
    warnings: &mut Vec<String>,
) {
    if !value.is_finite() {
        push_warning(
            warnings,
            format!("{path} is not a finite number; using the default {default}"),
        );
        *value = default;
    } else if *value < min {
        push_warning(
            warnings,
            format!("{path} {value} is below the supported minimum {min}; clamped to {min}"),
        );
        *value = min;
    } else if *value > max {
        push_warning(
            warnings,
            format!("{path} {value} exceeds the supported maximum {max}; clamped to {max}"),
        );
        *value = max;
    }
}

fn clamp_f64(
    value: &mut f64,
    min: f64,
    max: f64,
    default: f64,
    path: &str,
    warnings: &mut Vec<String>,
) {
    if !value.is_finite() {
        push_warning(
            warnings,
            format!("{path} is not a finite number; using the default {default}"),
        );
        *value = default;
    } else if *value < min {
        push_warning(
            warnings,
            format!("{path} {value} is below the supported minimum {min}; clamped to {min}"),
        );
        *value = min;
    } else if *value > max {
        push_warning(
            warnings,
            format!("{path} {value} exceeds the supported maximum {max}; clamped to {max}"),
        );
        *value = max;
    }
}

/// Single place where loaded configuration values are normalized and bounded.
///
/// Every load path (TOML, salvage recovery, Fastfetch import) funnels through
/// this function so no module ever sees a hostile value such as `padding = 99999`,
/// a NaN float or an unknown `shading_mode`.
fn normalize_config_values(cfg: &mut Config, warnings: &mut Vec<String>) {
    clamp_usize(&mut cfg.general.padding, 0, 64, "general.padding", warnings);
    clamp_usize(
        &mut cfg.general.box_padding,
        0,
        16,
        "general.box_padding",
        warnings,
    );
    clamp_usize(
        &mut cfg.general.colors.rows,
        1,
        2,
        "general.colors.rows",
        warnings,
    );
    clamp_usize(
        &mut cfg.general.logo.image_width_cols,
        1,
        256,
        "general.logo.image_width_cols",
        warnings,
    );

    clamp_f32(
        &mut cfg.general.logo.three_d.speed,
        0.05,
        10.0,
        1.0,
        "general.logo.three_d.speed",
        warnings,
    );
    clamp_f32(
        &mut cfg.general.logo.three_d.size,
        0.1,
        3.0,
        1.0,
        "general.logo.three_d.size",
        warnings,
    );
    clamp_f32(
        &mut cfg.general.logo.three_d.depth,
        0.1,
        5.0,
        1.0,
        "general.logo.three_d.depth",
        warnings,
    );
    clamp_optional_usize(
        &mut cfg.general.logo.three_d.width,
        1,
        512,
        "general.logo.three_d.width",
        warnings,
    );
    clamp_optional_usize(
        &mut cfg.general.logo.three_d.height,
        1,
        512,
        "general.logo.three_d.height",
        warnings,
    );
    clamp_f64(
        &mut cfg.general.logo.animation.fps,
        1.0,
        120.0,
        15.0,
        "general.logo.animation.fps",
        warnings,
    );

    clamp_optional_usize(
        &mut cfg.general.border_image_width,
        1,
        512,
        "general.border_image_width",
        warnings,
    );
    clamp_optional_usize(
        &mut cfg.general.border_image_height,
        1,
        512,
        "general.border_image_height",
        warnings,
    );
    clamp_optional_i32(
        &mut cfg.general.border_image_offset_x,
        -10_000,
        10_000,
        "general.border_image_offset_x",
        warnings,
    );
    clamp_optional_i32(
        &mut cfg.general.border_image_offset_y,
        -10_000,
        10_000,
        "general.border_image_offset_y",
        warnings,
    );
    clamp_optional_i32(
        &mut cfg.general.border_image_z_index,
        -100,
        100,
        "general.border_image_z_index",
        warnings,
    );

    for (index, image) in cfg.general.border_images.iter_mut().enumerate() {
        let prefix = format!("general.border_images[{index}]");
        clamp_optional_usize(
            &mut image.width,
            1,
            512,
            &format!("{prefix}.width"),
            warnings,
        );
        clamp_optional_usize(
            &mut image.height,
            1,
            512,
            &format!("{prefix}.height"),
            warnings,
        );
        clamp_optional_i32(
            &mut image.offset_x,
            -10_000,
            10_000,
            &format!("{prefix}.offset_x"),
            warnings,
        );
        clamp_optional_i32(
            &mut image.offset_y,
            -10_000,
            10_000,
            &format!("{prefix}.offset_y"),
            warnings,
        );
        clamp_i32(
            &mut image.z_index,
            -100,
            100,
            &format!("{prefix}.z_index"),
            warnings,
        );
    }

    for module in &mut cfg.modules {
        if let Some(width) = &mut module.bar_width {
            clamp_usize(
                width,
                1,
                256,
                &format!("module '{}'.bar_width", module.name),
                warnings,
            );
        }
        if let Some(style) = module.bar_style.take() {
            let trimmed = style.trim();
            if trimmed.is_empty() {
                continue;
            }
            let normalized = trimmed.to_lowercase();
            if is_known_bar_style(&normalized) {
                module.bar_style = Some(normalized);
            } else {
                push_warning(
                    warnings,
                    format!(
                        "module '{}' has unknown bar_style '{}'; expected one of {}",
                        module.name,
                        trimmed,
                        BAR_STYLES.join(", ")
                    ),
                );
            }
        }
    }

    let shading = cfg.general.logo.three_d.shading_mode.trim().to_lowercase();
    if is_known_shading_mode(&shading) {
        cfg.general.logo.three_d.shading_mode = shading;
    } else {
        push_warning(
            warnings,
            format!(
                "unknown shading_mode '{}'; expected one of {}; using 'ascii'",
                cfg.general.logo.three_d.shading_mode.trim(),
                SHADING_MODES.join(", ")
            ),
        );
        cfg.general.logo.three_d.shading_mode = "ascii".to_string();
    }
}

/// Collects non-clamping diagnostics about the current configuration values.
fn collect_config_warnings(cfg: &Config, warnings: &mut Vec<String>) {
    for module in &cfg.modules {
        let name = module.name.trim();
        if name.is_empty() {
            push_warning(
                warnings,
                "module entry without a name is ignored".to_string(),
            );
        } else if !is_known_module_name(name) {
            push_warning(
                warnings,
                format!(
                    "unknown module '{name}' is ignored; run `rustfetch --list-modules` for the supported names"
                ),
            );
        }
        if let Some(style) = &module.bar_style {
            if !is_known_bar_style(style) {
                push_warning(
                    warnings,
                    format!(
                        "module '{name}' has unknown bar_style '{}'; expected one of {}",
                        style.trim(),
                        BAR_STYLES.join(", ")
                    ),
                );
            }
        }
    }

    if let Some(theme) = &cfg.general.theme {
        let trimmed = theme.trim();
        if !trimmed.is_empty() && !is_known_theme_name(trimmed) {
            push_warning(
                warnings,
                format!("unknown theme '{trimmed}' (ignored by the renderer)"),
            );
        }
    }

    if !is_known_shading_mode(&cfg.general.logo.three_d.shading_mode) {
        push_warning(
            warnings,
            format!(
                "unknown shading_mode '{}'; expected one of {}",
                cfg.general.logo.three_d.shading_mode.trim(),
                SHADING_MODES.join(", ")
            ),
        );
    }
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
    pub fn from_preset(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "minimal" => {
                let mut c = Config::default();
                c.general.icons = false;
                c.general.separator = " :".to_string();
                c.general.colors.enabled = false;
                c.general.logo.enabled = false;
                c.modules =
                    modules_from(&["os", "kernel", "uptime", "shell", "cpu", "memory", "disk"]);
                Some(c)
            }
            "clean" => {
                let mut c = Config::default();
                c.general.icon_only = true;
                c.general.separator = "".to_string();
                c.general.padding = 1;
                c.general.colors.enabled = false;
                c.modules = modules_from(&[
                    "os", "kernel", "desktop", "shell", "terminal", "cpu", "gpu", "memory",
                    "uptime",
                ]);
                Some(c)
            }
            "dots" => {
                let mut c = Config::default();
                c.general.icon_only = true;
                c.general.separator = "•".to_string();
                c.general.padding = 1;
                c.general.colors.enabled = true;
                c.general.colors.symbol = "●".to_string();
                c.modules = modules_from(&[
                    "os", "kernel", "uptime", "shell", "terminal", "cpu", "gpu", "memory", "disk",
                ]);
                Some(c)
            }
            "card" | "modern-card" => {
                let mut c = Config::default();
                c.general.border = true;
                c.general.border_style = Some("rounded".to_string());
                c.general.padding = 2;
                c.general.center = true;
                c.general.colors.enabled = true;
                c.general.colors.position = "bottom".to_string();
                c.modules = modules_from(&[
                    "os", "host", "kernel", "uptime", "break", "shell", "terminal", "desktop",
                    "break", "cpu", "gpu", "memory", "disk",
                ]);
                Some(c)
            }
            "brackets" => {
                let mut c = Config::default();
                c.general.border = true;
                c.general.border_style = Some("brackets".to_string());
                c.general.padding = 2;
                c.general.center = true;
                Some(c)
            }
            "neofetch" => {
                let mut c = Config::default();
                c.general.separator = ":".to_string();
                c.general.padding = 1;
                c.general.colors.enabled = true;
                c.general.colors.block = true;
                c.general.colors.rows = 2;
                c.general.colors.position = "bottom".to_string();
                Some(c)
            }
            "retro" => {
                let mut c = Config::default();
                c.general.border = true;
                c.general.border_style = Some("rounded".to_string());
                c.general.colors.symbol = "󰮯".to_string();
                for m in &mut c.modules {
                    if matches!(m.name.as_str(), "memory" | "disk" | "battery") {
                        m.bar = Some(true);
                        m.bar_width = Some(10);
                    }
                }
                Some(c)
            }
            "modern" => {
                let mut c = Config::default();
                c.general.center = true;
                c.general.separator = " ->".to_string();
                c.general.colors.block = true;
                for m in &mut c.modules {
                    if matches!(m.name.as_str(), "memory" | "swap" | "disk" | "battery") {
                        m.bar = Some(true);
                        m.bar_width = Some(10);
                    }
                }
                Some(c)
            }
            "compact" => {
                let mut c = Config::default();
                c.general.padding = 1;
                c.general.colors.enabled = false;
                c.modules =
                    modules_from(&["os", "host", "kernel", "uptime", "shell", "cpu", "memory"]);
                Some(c)
            }
            "default" => Some(Config::default()),
            _ => None,
        }
    }

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

    pub fn find_fastfetch_config() -> Option<PathBuf> {
        let check_dir = |dir: PathBuf| {
            let jsonc = dir.join("fastfetch").join("config.jsonc");
            if jsonc.is_file() {
                return Some(jsonc);
            }
            let json = dir.join("fastfetch").join("config.json");
            if json.is_file() {
                return Some(json);
            }
            None
        };

        if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
            let trimmed = xdg.trim();
            if !trimmed.is_empty() {
                if let Some(p) = check_dir(PathBuf::from(trimmed)) {
                    return Some(p);
                }
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let trimmed = home.trim();
            if !trimmed.is_empty() {
                if let Some(p) = check_dir(PathBuf::from(trimmed).join(".config")) {
                    return Some(p);
                }
            }
        }
        None
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
        {
            if let Some(cfg) = Self::import_from_fastfetch(&content) {
                return Ok(cfg);
            }
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

    /// Fault-tolerant recovery for older or partially invalid user `config.toml` files.
    /// If a single field has a deprecated or invalid value type, preserves all other valid
    /// user settings instead of discarding the entire configuration.
    fn salvage_partial_toml(content: &str) -> Option<Self> {
        let root: toml::Table = content.parse().ok()?;
        let mut cfg = Config::default();
        let mut recovered_any = false;

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
        }

        if let Some(toml::Value::Array(mods_arr)) = root.get("modules") {
            let mut valid_mods = Vec::new();
            for item in mods_arr {
                if let Ok(mc) = ModuleConfig::deserialize(item.clone()) {
                    if !mc.name.trim().is_empty() {
                        valid_mods.push(mc);
                    }
                }
            }
            if !valid_mods.is_empty() {
                cfg.modules = valid_mods;
                recovered_any = true;
            }
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

    pub fn to_default_toml() -> String {
        r##"# ==============================================================================
# RustFetch Configuration (config.toml)
# ==============================================================================
# Blazing-fast, highly customizable system information tool written in Rust.
# Edit this file to customize layout, borders, overlays, colors, and modules.
# ==============================================================================

# ------------------------------------------------------------------------------
# 1. General Display & Layout
# ------------------------------------------------------------------------------
[general]
# Separator between module labels and values (e.g. ":", ": ", "  ", " ")
separator = ":"

# Spaces between separator and value
padding = 1

# Center output horizontally in terminal
center = false

# Enable/disable Nerd Font icons next to module labels
icons = true

# Display only icons and values (hide module label text)
icon_only = false

# Run in live interactive system monitor mode (continuous refresh)
live = false

# Check GitHub repository for updates and display an Update row at the bottom when available
check_updates = true

# Preset theme: "catppuccin", "nord", "dracula", "gruvbox", "tokyonight", "rosepine", "solarized", "monokai", "cyberpunk"
# theme = "catppuccin"

# Multi-stop color gradient for text and borders (list of hex codes)
# gradient = ["#89b4fa", "#b4befe", "#cba6f7"]

# ------------------------------------------------------------------------------
# 2. Border & Box Frame
# ------------------------------------------------------------------------------
[general.border]
# Enable or disable the outer border box
enabled = false

# Border frame style: "rounded", "single", "double", "thick", "dashed", "block", "ascii", "brackets"
# style = "rounded"

# Border color: color name ("cyan", "red", "green", "blue", "magenta", "yellow") or hex ("#89b4fa")
# color = "cyan"

# Optional title displayed in the top border frame
# title = "RustFetch"

# Padding (in spaces) between border frame and content
padding = 1

# Custom border characters (overrides style if set)
# [general.border.chars]
# top_left = "╭"
# top = "─"
# top_right = "╮"
# right = "│"
# bottom_right = "╯"
# bottom = "─"
# bottom_left = "╰"
# left = "│"
# divider_left = "├"
# divider = "─"
# divider_right = "┤"

# ------------------------------------------------------------------------------
# 3. Border Image & Background Overlays
# ------------------------------------------------------------------------------
[general.border_image]
# Position options: "behind", "background", "left", "right", "top", "bottom"
# Default is "behind" (render image as background backdrop)
position = "behind"

# Overlay an image behind or alongside the fetch layout
# Supported protocols: "auto", "kitty", "sixel", "iterm", "halfblocks"
# path = "~/.config/rustfetch/background.png"
# width = 30
# height = 18
# offset_x = 0
# offset_y = 0
# z_index = -1
# protocol = "auto"

# Multiple layered border images (optional)
# [[general.border_images]]
# path = "~/.config/rustfetch/badge.png"
# position = "right"
# width = 20
# height = 10
# offset_x = 0
# offset_y = 0
# z_index = 1
# protocol = "auto"

# ------------------------------------------------------------------------------
# 4. Color Palette Bar
# ------------------------------------------------------------------------------
[general.colors]
# Display color test dots/blocks at bottom or top of output
enabled = true

# Character used for color blocks (e.g. "●", "■", "󰝤", "")
symbol = "●"

# Use solid colored rectangular blocks instead of symbols
block = false

# Extract color palette automatically from image logo when image mode is used
image_palette = true

# Position of the palette: "bottom", "top", "none"
position = "bottom"

# Number of rows (1 or 2 rows; 2 rows displays standard + bright colors)
rows = 1

# Custom color palette override (hex codes or ANSI color names)
# custom = ["#1e1e2e", "#f38ba8", "#a6e3a1", "#f9e2af", "#89b4fa", "#cba6f7", "#94e2d5", "#cdd6f4"]

# ------------------------------------------------------------------------------
# 5. Logo & Visual Artwork
# ------------------------------------------------------------------------------
[general.logo]
# Enable or disable logo display
enabled = true

# Logo placement: "left" (default), "right", "top", "bottom", "behind", "hidden"
# position = "left"

# OS distro logo to display ("auto" detects current OS, or e.g. "arch", "ubuntu", "fedora", "nixos")
distro = "auto"

# Custom ASCII logo file path (overrides built-in distro art)
# ascii_path = "~/.config/rustfetch/ascii_logo.txt"

# Custom colors for multi-color ASCII logos (e.g. ["blue", "white"])
# colors = ["blue", "white"]

# Image logo file path (PNG, JPG, WebP)
# image_path = "~/.config/rustfetch/logo.png"

# Directory to randomly pick an image logo from on each launch
# image_dir = "~/.config/rustfetch/logos"

# Width of image logo in terminal columns
image_width_cols = 42

# Randomly select image from image_dir
random_image = false

# Automatically adapt primary theme colors to match dominant image colors
auto_color = true

# Terminal graphics protocol: "auto", "kitty", "sixel", "iterm", "halfblocks"
protocol = "auto"

# ------------------------------------------------------------------------------
# 6. 3D Spinning ASCII Relief Logo
# ------------------------------------------------------------------------------
[general.logo.three_d]
# Enable real-time animated 3D ASCII relief logo
enabled = false

# Rotation speed multiplier (default: 1.0)
speed = 1.0

# Enable rotation around X axis (pitch / wobble)
rotate_x = true

# Enable rotation around Y axis (horizontal yaw spin)
rotate_y = true

# Model size scaling factor
size = 1.0

# 3D depth extrusion factor
depth = 1.0

# Canvas dimensions in characters
width = 42
height = 24

# Shading mode: "ascii", "dot", "wireframe", "solid", "halfblock", "braille"
shading_mode = "ascii"

# Custom shading characters from darkest to brightest (default: " .:-=+*#%@")
# shading = " .:-=+*#%@"

# Simulated directional light source: "top-left", "top-right", "front", "top"
light = "top-left"

# Stop animation after N frames (default: None = run indefinitely)
# frames = 120

# Loop animation indefinitely
# infinite = true

# Hold terminal open and keep animating until Ctrl+C / 'q' (disables exit-on-key)
# hold = false

# Automatically exit animation on any keypress (perfect for instant shell access)
exit_on_key = true

# Custom outer and inner colors for 3D model (hex or color name)
# outer_color = "#89b4fa"
# inner_color = "#cdd6f4"

# ------------------------------------------------------------------------------
# 7. Multi-frame ASCII Animation
# ------------------------------------------------------------------------------
[general.logo.animation]
# Enable multi-frame ASCII animation
enabled = false

# Path to multi-frame ASCII animation text file (delimited by '---' or form-feed)
# path = "~/.config/rustfetch/anim.txt"

# Frames per second (FPS)
fps = 15.0

# Exit animation on any keypress
exit_on_key = true

# Loop animation indefinitely
infinite = true

# Hold animation open until Ctrl+C / 'q'
# hold = false

# Stop animation after N frames
# frames = 60

# ==============================================================================
# 8. Information Modules
# ==============================================================================
# Configure which modules are shown, their order, custom labels, icons, and formats.
#
# Available modules:
#   os, os_age, host, board, kernel, uptime, packages, shell, desktop, font,
#   terminal, cpu, cpu_usage, temp, gpu, dgpu, igpu, gpu_driver, memory, swap, disk,
#   battery, power_profile, brightness, display, sound, wifi, net_io,
#   local_ip, locale, processes, media, init, container, update,
#   break (blank spacer line), divider (horizontal rule), custom
#
# Optional per-module fields:
#   label     - Custom display label text (or "" to hide label)
#   icon      - Custom Nerd Font icon
#   color     - Module label color (hex or ANSI name)
#   format    - Value template (e.g. "{value}", "{}")
#   bar       - true/false: display percentage progress bar
#   bar_width - Width of progress bar in characters (default: 10)
#   path      - Mount point for disk module (e.g. path = "/home")
#   type      - GPU filter for gpu module: "dgpu" (priority dGPU), "igpu" (integrated only), "all" (both)
#   value     - Static text value (for name = "custom")

[[modules]]
name = "os"

[[modules]]
name = "host"

[[modules]]
name = "board"

[[modules]]
name = "kernel"

[[modules]]
name = "uptime"

[[modules]]
name = "packages"

[[modules]]
name = "shell"

[[modules]]
name = "desktop"

[[modules]]
name = "font"

[[modules]]
name = "terminal"

[[modules]]
name = "cpu"

[[modules]]
name = "gpu"

[[modules]]
name = "memory"

[[modules]]
name = "swap"

[[modules]]
name = "disk"

[[modules]]
name = "battery"

[[modules]]
name = "local_ip"

[[modules]]
name = "locale"
"##
        .to_string()
    }

    pub fn to_toml_string(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    pub fn import_from_fastfetch(content: &str) -> Option<Self> {
        let stripped = strip_jsonc(content);
        let val: serde_json::Value = serde_json::from_str(&stripped).ok()?;

        let mut config = Self::default();

        if let Some(sep) = val.pointer("/display/separator").and_then(|s| s.as_str()) {
            if sep == " " {
                config.general.separator = " ".to_string();
            } else {
                config.general.separator = sep.trim_end().to_string();
            }
        }

        if let Some(pad) = val
            .pointer("/display/key/paddingLeft")
            .and_then(|p| p.as_u64())
        {
            config.general.padding = pad as usize;
        }

        let mut center_detected = false;
        if let Some(logo) = val.get("logo").and_then(|l| l.as_object()) {
            if let Some(ltype) = logo.get("type").and_then(|t| t.as_str()) {
                if ltype == "none" {
                    config.general.logo.enabled = false;
                } else if ltype == "kitty" {
                    config.general.logo.protocol = "kitty".to_string();
                }
            }

            if let Some(pad_left) = logo
                .get("padding")
                .and_then(|p| p.get("left"))
                .and_then(|v| v.as_i64())
            {
                if pad_left > 0 {
                    center_detected = true;
                }
            }

            if let Some(width) = logo.get("width").and_then(|w| w.as_u64()) {
                if (15..=60).contains(&width) {
                    config.general.logo.image_width_cols = width.min(45) as usize;
                }
            }

            if let Some(src) = logo.get("source").and_then(|s| s.as_str()) {
                if !src.is_empty() {
                    let is_file_type = logo.get("type").and_then(|t| t.as_str()) == Some("file");
                    let path = PathBuf::from(src);
                    if is_file_type {
                        config.general.logo.ascii_path = Some(src.to_string());
                    } else if let Some(parent) = path.parent() {
                        let parent_str = parent.to_string_lossy();
                        if parent_str.ends_with("/logos") || parent_str.contains("/logos/") {
                            config.general.logo.image_dir = Some(parent_str.to_string());
                            config.general.logo.random_image = true;
                            config.general.logo.auto_color = true;
                        } else if path.is_dir() {
                            config.general.logo.image_dir = Some(src.to_string());
                            config.general.logo.random_image = true;
                            config.general.logo.auto_color = true;
                        } else {
                            config.general.logo.image_path = Some(src.to_string());
                            config.general.logo.auto_color = true;
                        }
                    } else {
                        config.general.logo.image_path = Some(src.to_string());
                    }
                }
            }
        }

        if config.general.logo.image_dir.is_none() && config.general.logo.image_path.is_none() {
            if let Ok(home) = std::env::var("HOME") {
                let ff_logos = PathBuf::from(home).join(".config/fastfetch/logos");
                if ff_logos.is_dir() {
                    config.general.logo.image_dir = Some("~/.config/fastfetch/logos".to_string());
                    config.general.logo.random_image = true;
                    config.general.logo.auto_color = true;
                }
            }
        }

        if let Some(modules_arr) = val.get("modules").and_then(|m| m.as_array()) {
            let mut converted = Vec::new();
            let mut seen = std::collections::HashSet::new();

            for item in modules_arr {
                if let Some(mod_obj) = item.as_object() {
                    let mod_type = mod_obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if mod_type == "custom" {
                        if let Some(fmt) = mod_obj.get("format").and_then(|f| f.as_str()) {
                            if fmt.contains('\n') {
                                center_detected = true;
                            }
                        }
                        let mut mc = ModuleConfig::new("custom");
                        if let Some(key) = mod_obj.get("key").and_then(|k| k.as_str()) {
                            mc.label = Some(key.to_string());
                        }
                        if let Some(fmt) = mod_obj.get("format").and_then(|f| f.as_str()) {
                            mc.value = Some(fmt.to_string());
                        }
                        if mc.label.is_some() || mc.value.is_some() {
                            converted.push(mc);
                        }
                        continue;
                    }

                    if let Some(mapped_name) = map_fastfetch_type(mod_type) {
                        let allow_multiple = matches!(
                            mapped_name,
                            "break"
                                | "custom"
                                | "divider"
                                | "packages"
                                | "gpu"
                                | "dgpu"
                                | "igpu"
                                | "disk"
                        );
                        if allow_multiple || seen.insert(mapped_name) {
                            let mut mc = ModuleConfig::new(mapped_name);
                            if let Some(icon) = mod_obj.get("keyIcon").and_then(|i| i.as_str()) {
                                if !icon.trim().is_empty() {
                                    mc.icon = Some(icon.to_string());
                                }
                            }
                            if let Some(key) = mod_obj.get("key").and_then(|k| k.as_str()) {
                                if !key.trim().is_empty() {
                                    mc.label = Some(key.to_string());
                                }
                            }
                            if let Some(color) = mod_obj.get("keyColor").and_then(|c| c.as_str()) {
                                mc.color = Some(color.to_string());
                            }
                            if let Some(fmt) = mod_obj.get("format").and_then(|f| f.as_str()) {
                                mc.format = Some(fmt.to_string());
                            }
                            converted.push(mc);
                        }
                    }
                } else if let Some(mod_name) = item.as_str() {
                    if let Some(mapped_name) = map_fastfetch_type(mod_name) {
                        let allow_multiple = matches!(
                            mapped_name,
                            "break"
                                | "custom"
                                | "divider"
                                | "packages"
                                | "gpu"
                                | "dgpu"
                                | "igpu"
                                | "disk"
                        );
                        if allow_multiple || seen.insert(mapped_name) {
                            converted.push(ModuleConfig::new(mapped_name));
                        }
                    }
                }
            }

            if !converted.is_empty() {
                config.modules = converted;
            }
        }

        config.general.center = center_detected;

        let mut warnings = Vec::new();
        normalize_config_values(&mut config, &mut warnings);
        config.load_warnings = warnings;
        Some(config)
    }
}

fn map_fastfetch_type(ff_type: &str) -> Option<&'static str> {
    match ff_type {
        "os" => Some("os"),
        "host" => Some("host"),
        "board" => Some("board"),
        "kernel" => Some("kernel"),
        "uptime" => Some("uptime"),
        "packages" => Some("packages"),
        "shell" => Some("shell"),
        "de" | "wm" | "theme" | "wmtheme" | "icons" | "cursor" => Some("desktop"),
        "font" | "terminalfont" => Some("font"),
        "terminal" => Some("terminal"),
        "cpu" => Some("cpu"),
        "cpu_usage" | "cpuusage" => Some("cpu_usage"),
        "gpu" => Some("gpu"),
        "dgpu" => Some("dgpu"),
        "igpu" => Some("igpu"),
        "memory" => Some("memory"),
        "swap" => Some("swap"),
        "disk" => Some("disk"),
        "battery" | "poweradapter" => Some("battery"),
        "localip" | "local_ip" => Some("local_ip"),
        "locale" => Some("locale"),
        "break" | "divider" | "separator" => Some("break"),
        "custom" => Some("custom"),
        "media" | "player" => Some("media"),
        "sound" | "audio" => Some("sound"),
        "wifi" => Some("wifi"),
        "bluetooth" => Some("bluetooth"),
        "brightness" => Some("brightness"),
        "processes" => Some("processes"),
        "temp" | "temperature" => Some("temp"),
        "init" | "initsystem" => Some("init"),
        "container" | "vm" => Some("container"),
        "os_age" | "age" | "disk_age" => Some("os_age"),
        "power_profile" | "powerprofile" | "power" => Some("power_profile"),
        "gpu_driver" | "gpudriver" | "vulkan" | "opengl" => Some("gpu_driver"),
        "net_io" | "netio" | "network" => Some("net_io"),
        _ => None,
    }
}

pub fn strip_jsonc(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(n);
    let mut i = 0;
    let mut in_str = false;
    let mut escape = false;

    while i < n {
        let c = chars[i];
        if in_str {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if c == '/' && i + 1 < n {
            if chars[i + 1] == '/' {
                i += 2;
                while i < n && chars[i] != '\n' {
                    i += 1;
                }
                if i < n {
                    out.push('\n');
                }
            } else if chars[i + 1] == '*' {
                i += 2;
                while i + 1 < n && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 1;
            } else {
                out.push(c);
            }
        } else {
            out.push(c);
        }
        i += 1;
    }

    let out_chars: Vec<char> = out.chars().collect();
    let len = out_chars.len();
    let mut final_out = String::with_capacity(len);
    let mut j = 0;
    in_str = false;
    escape = false;

    while j < len {
        let c = out_chars[j];
        if in_str {
            final_out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            final_out.push(c);
        } else if c == ',' {
            let mut k = j + 1;
            while k < len && out_chars[k].is_whitespace() {
                k += 1;
            }
            if k < len && (out_chars[k] == '}' || out_chars[k] == ']') {
                // skip trailing comma
            } else {
                final_out.push(c);
            }
        } else {
            final_out.push(c);
        }
        j += 1;
    }

    final_out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_has_all_modules() {
        let config = Config::default();
        assert_eq!(config.modules.len(), 18);

        let names: Vec<&str> = config.modules.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "os", "host", "board", "kernel", "uptime", "packages", "shell", "desktop", "font",
                "terminal", "cpu", "gpu", "memory", "swap", "disk", "battery", "local_ip",
                "locale"
            ]
        );

        assert_eq!(config.general.separator, ":");
        assert_eq!(config.general.padding, 1);
        assert!(config.general.logo.enabled);
        assert_eq!(config.general.logo.distro, "auto");

        for m in &config.modules {
            assert!(m.label.is_none());
            assert!(m.icon.is_none());
            assert!(m.color.is_none());
        }
    }

    #[test]
    fn test_parse_toml_string() {
        let toml_str = r#"
[general]
separator = " ->"
padding = 4

[general.logo]
enabled = false
distro = "fedora"

[[modules]]
name = "os"
label = "Distro"
icon = ""
color = "blue"

[[modules]]
name = "cpu"
icon = "󰍛"
"#;

        let config: Config = Config::from_toml(toml_str).expect("failed to parse valid TOML");
        assert_eq!(config.general.separator, " ->");
        assert_eq!(config.general.padding, 4);
        assert!(!config.general.logo.enabled);
        assert_eq!(config.general.logo.distro, "fedora");

        assert_eq!(config.modules.len(), 2);
        assert_eq!(config.modules[0].name, "os");
        assert_eq!(config.modules[0].label.as_deref(), Some("Distro"));
        assert_eq!(config.modules[0].icon.as_deref(), Some(""));
        assert_eq!(config.modules[0].color.as_deref(), Some("blue"));

        assert_eq!(config.modules[1].name, "cpu");
        assert_eq!(config.modules[1].label, None);
        assert_eq!(config.modules[1].icon.as_deref(), Some("󰍛"));
        assert_eq!(config.modules[1].color, None);
    }

    #[test]
    fn test_parse_partial_toml_uses_defaults() {
        let toml_str = r#"
[general]
separator = " >"
"#;
        let config: Config = Config::from_toml(toml_str).expect("failed to parse partial TOML");
        assert_eq!(config.general.separator, " >");
        assert_eq!(config.general.padding, 1);
        assert!(config.general.logo.enabled);
        assert_eq!(config.general.logo.distro, "auto");
        assert_eq!(config.modules.len(), 18);
        assert_eq!(config.modules[0].name, "os");
    }

    #[test]
    fn test_invalid_toml_gives_error() {
        let bad_syntax = "general = [unclosed bracket";
        let err = Config::from_toml(bad_syntax);
        assert!(err.is_err());
        let err_msg = err.unwrap_err().to_string();
        assert!(
            err_msg.contains("line") || err_msg.contains("error"),
            "Error message should mention location or error: {err_msg}"
        );

        let bad_type = r#"
[general]
separator = 12345
"#;
        let err2 = Config::from_toml(bad_type);
        assert!(err2.is_err());
        let err2_msg = err2.unwrap_err().to_string();
        assert!(
            err2_msg.contains("invalid type") || err2_msg.contains("expected a string"),
            "Error message should explain what was expected: {err2_msg}"
        );
    }

    #[test]
    fn test_to_default_toml_round_trips() {
        let default_config = Config::default();
        let toml_str = Config::to_default_toml();

        assert!(!toml_str.is_empty());
        assert!(toml_str.contains("separator = \":\""));
        assert!(toml_str.contains("padding = 1"));
        assert!(toml_str.contains("enabled = true"));
        assert!(toml_str.contains("distro = \"auto\""));
        assert!(toml_str.contains("name = \"os\""));
        assert!(toml_str.contains("name = \"kernel\""));

        let deserialized: Config =
            Config::from_toml(&toml_str).expect("round-trip deserialization failed");
        assert_eq!(deserialized, default_config);
    }

    #[test]
    fn test_config_dir_precedence() {
        let dir1 = Config::config_dir_from_env(|var| match var {
            "XDG_CONFIG_HOME" => Some("/custom/config".into()),
            "HOME" => Some("/home/user".into()),
            _ => None,
        });
        assert_eq!(dir1, PathBuf::from("/custom/config"));

        let dir2 = Config::config_dir_from_env(|var| match var {
            "XDG_CONFIG_HOME" => Some("   ".into()),
            "HOME" => Some("/home/user".into()),
            _ => None,
        });
        assert_eq!(dir2, PathBuf::from("/home/user/.config"));

        let dir3 = Config::config_dir_from_env(|var| match var {
            "XDG_CONFIG_HOME" => None,
            "HOME" => Some("/home/user".into()),
            _ => None,
        });
        assert_eq!(dir3, PathBuf::from("/home/user/.config"));

        let dir4 = Config::config_dir_from_env(|_| None);
        assert_eq!(dir4, PathBuf::from(".config"));
        assert!(!dir4.to_string_lossy().contains('~'));

        // config_path() must build on config_dir() and never emit a literal tilde.
        let path = Config::config_path();
        assert!(path.ends_with("rustfetch/config.toml"));
        assert!(!path.to_string_lossy().contains('~'));
    }

    #[test]
    fn test_load_from_file_and_errors() {
        let temp_dir = std::env::temp_dir().join(format!("rustfetch_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&temp_dir);

        let valid_file = temp_dir.join("valid.toml");
        std::fs::write(
            &valid_file,
            "[general]\npadding = 3\nseparator = \" -> \"\n",
        )
        .expect("write file");

        let loaded = Config::load_from(&valid_file).expect("load_from valid file");
        assert_eq!(loaded.general.padding, 3);
        assert_eq!(loaded.general.separator, " -> ");

        let invalid_file = temp_dir.join("invalid.toml");
        std::fs::write(&invalid_file, "[general]\npadding = \"not_a_number\"\n")
            .expect("write file");

        let err = Config::load_from(&invalid_file).unwrap_err();
        assert!(err.contains("Failed to parse config file"));
        assert!(err.contains("line"));

        let nonexistent = temp_dir.join("nonexistent.toml");
        let not_found_err = Config::load_from(&nonexistent).unwrap_err();
        assert!(not_found_err.contains("Failed to read config file"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_strip_jsonc_comments_and_trailing_commas() {
        let input = r#"{
            // This is a line comment
            "name": "https://example.com/test//not_comment",
            /* This is a
               block comment */
            "items": [
                1,
                2,
                3, // trailing comma here
            ],
            "enabled": true,
        }"#;

        let cleaned = strip_jsonc(input);
        assert!(!cleaned.contains("This is a line comment"));
        assert!(!cleaned.contains("block comment"));
        assert!(cleaned.contains("https://example.com/test//not_comment"));

        let parsed: serde_json::Value =
            serde_json::from_str(&cleaned).expect("should parse valid json");
        assert_eq!(parsed["name"], "https://example.com/test//not_comment");
        assert_eq!(parsed["items"].as_array().unwrap().len(), 3);
        assert_eq!(parsed["enabled"], true);
    }

    #[test]
    fn test_import_from_fastfetch() {
        let ff_jsonc = r#"{
            "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/master/doc/json_schema.json",
            "logo": {
                "type": "kitty",
                "source": "~/.config/fastfetch/logos/current.png",
                "width": 60,
                "padding": {
                    "left": 47
                }
            },
            "display": {
                "separator": ": "
            },
            "modules": [
                {
                    "type": "custom",
                    "format": "\n\n\n\n\n\n"
                },
                {
                    "type": "title",
                    "keyIcon": ""
                },
                {
                    "type": "separator"
                },
                {
                    "type": "os",
                    "keyIcon": ""
                },
                {
                    "type": "cpu",
                    "keyIcon": ""
                },
                {
                    "type": "de",
                    "keyIcon": ""
                },
                {
                    "type": "wm",
                    "keyIcon": ""
                },
                {
                    "type": "localip",
                    "keyIcon": "󰩟"
                }
            ]
        }"#;

        let config =
            Config::import_from_fastfetch(ff_jsonc).expect("should import fastfetch config");
        assert_eq!(config.general.separator, ":");
        assert!(config.general.center);
        assert!(config.general.logo.random_image);
        assert_eq!(
            config.general.logo.image_dir.as_deref(),
            Some("~/.config/fastfetch/logos")
        );

        let mod_names: Vec<&str> = config.modules.iter().map(|m| m.name.as_str()).collect();
        // custom and break preserved, de and wm deduplicated to a single desktop module
        assert_eq!(
            mod_names,
            vec!["custom", "break", "os", "cpu", "desktop", "local_ip"]
        );

        let os_mod = config.modules.iter().find(|m| m.name == "os").unwrap();
        assert_eq!(os_mod.icon.as_deref(), Some(""));

        let cpu_mod = config.modules.iter().find(|m| m.name == "cpu").unwrap();
        assert_eq!(cpu_mod.icon.as_deref(), Some(""));

        let desktop_mod = config.modules.iter().find(|m| m.name == "desktop").unwrap();
        assert_eq!(desktop_mod.icon.as_deref(), Some(""));

        let local_ip_mod = config
            .modules
            .iter()
            .find(|m| m.name == "local_ip")
            .unwrap();
        assert_eq!(local_ip_mod.icon.as_deref(), Some("󰩟"));
    }

    #[test]
    fn test_from_preset() {
        let minimal = Config::from_preset("minimal").unwrap();
        assert!(!minimal.general.icons);
        assert!(!minimal.general.logo.enabled);

        let card = Config::from_preset("card").unwrap();
        assert!(card.general.border);

        let modern = Config::from_preset("modern").unwrap();
        assert!(modern.general.colors.block);

        let compact = Config::from_preset("compact").unwrap();
        assert_eq!(compact.modules.len(), 7);

        assert!(Config::from_preset("nonexistent").is_none());
    }

    #[test]
    fn test_module_config_text_and_logo_aliases() {
        let toml_str = r#"
        [[modules]]
        name = "os"
        text = "System"
        logo = ">_"

        [[modules]]
        name = "cpu"
        title = "Processor"
        symbol = ""
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.modules[0].label.as_deref(), Some("System"));
        assert_eq!(config.modules[0].icon.as_deref(), Some(">_"));
        assert_eq!(config.modules[1].label.as_deref(), Some("Processor"));
        assert_eq!(config.modules[1].icon.as_deref(), Some(""));
    }

    #[test]
    fn test_three_d_config_boolean_and_table() {
        let toml_bool = r#"
        [general.logo]
        3d = true
        "#;
        let cfg1: Config = toml::from_str(toml_bool).unwrap();
        assert!(cfg1.general.logo.three_d.enabled);

        let toml_table = r##"
        [general.logo.three_d]
        enabled = true
        speed = 2.0
        width = 60
        outer_color = "#3b82f6"
        "##;
        let cfg2: Config = toml::from_str(toml_table).unwrap();
        assert!(cfg2.general.logo.three_d.enabled);
        assert_eq!(cfg2.general.logo.three_d.speed, 2.0);
        assert_eq!(cfg2.general.logo.three_d.width, Some(60));
        assert_eq!(
            cfg2.general.logo.three_d.outer_color.as_deref(),
            Some("#3b82f6")
        );

        let toml_gen = r#"
        [general]
        3d = true
        "#;
        let cfg3: Config = toml::from_str(toml_gen).unwrap();
        assert_eq!(cfg3.general.three_d, Some(true));
    }

    #[test]
    fn test_border_images_config_and_active_list() {
        let toml_str = r#"
        [general]
        border_image = "legacy_border.png"
        border_image_width = 30
        border_image_position = "left"

        [[general.border_images]]
        path = "card_backdrop.png"
        position = "background"
        width = 50
        height = 20
        offset_x = 2
        offset_y = -1
        z_index = -1
        protocol = "kitty"

        [[general.border_images]]
        path = "side_ornament.png"
        position = "right"
        "#;

        let cfg: Config = toml::from_str(toml_str).unwrap();
        let active = cfg.general.active_border_images();
        assert_eq!(active.len(), 3);

        // Legacy image normalized
        assert_eq!(active[0].path, "legacy_border.png");
        assert_eq!(active[0].position, "left");
        assert_eq!(active[0].width, Some(30));
        assert!(!active[0].is_background());

        // Multi-image 1 (background)
        assert_eq!(active[1].path, "card_backdrop.png");
        assert_eq!(active[1].position, "background");
        assert_eq!(active[1].width, Some(50));
        assert_eq!(active[1].height, Some(20));
        assert_eq!(active[1].offset_x, Some(2));
        assert_eq!(active[1].offset_y, Some(-1));
        assert_eq!(active[1].z_index, -1);
        assert_eq!(active[1].protocol, "kitty");
        assert!(active[1].is_background());

        // Multi-image 2 (right)
        assert_eq!(active[2].path, "side_ornament.png");
        assert_eq!(active[2].position, "right");
        assert_eq!(active[2].z_index, -1);
        assert!(!active[2].is_background());
    }

    #[test]
    fn test_grouped_border_and_border_image_tables() {
        let toml_str = r#"
        [general]
        separator = " => "

        [general.border]
        enabled = true
        style = "double"
        color = "magenta"
        title = "MyBox"
        padding = 2

        [general.border_image]
        path = "wallpaper.png"
        position = "behind"
        width = 40
        height = 20
        offset_x = 1
        offset_y = 2
        z_index = -2
        protocol = "sixel"
        "#;

        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.general.separator, " => ");
        assert!(cfg.general.border);
        assert_eq!(cfg.general.border_style.as_deref(), Some("double"));
        assert_eq!(cfg.general.border_color.as_deref(), Some("magenta"));
        assert_eq!(cfg.general.border_title.as_deref(), Some("MyBox"));
        assert_eq!(cfg.general.box_padding, 2);

        assert_eq!(cfg.general.border_image.as_deref(), Some("wallpaper.png"));
        assert_eq!(cfg.general.border_image_position, "behind");
        assert_eq!(cfg.general.border_image_width, Some(40));
        assert_eq!(cfg.general.border_image_height, Some(20));
        assert_eq!(cfg.general.border_image_offset_x, Some(1));
        assert_eq!(cfg.general.border_image_offset_y, Some(2));
        assert_eq!(cfg.general.border_image_z_index, Some(-2));
        assert_eq!(cfg.general.border_image_protocol.as_deref(), Some("sixel"));

        let active = cfg.general.active_border_images();
        assert_eq!(active.len(), 1);
        assert!(active[0].is_background());
    }

    #[test]
    fn test_top_level_border_and_border_image_tables() {
        let toml_str = r#"
        [general]
        padding = 3

        [border]
        enabled = true
        style = "thick"

        [border_image]
        path = "bg.png"
        "#;

        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.general.padding, 3);
        assert!(cfg.general.border);
        assert_eq!(cfg.general.border_style.as_deref(), Some("thick"));
        assert_eq!(cfg.general.border_image.as_deref(), Some("bg.png"));
        assert_eq!(cfg.general.border_image_position, "behind");
    }

    #[test]
    fn test_border_image_default_position_is_behind() {
        let cfg = Config::default();
        assert_eq!(cfg.general.border_image_position, "behind");
        let bi = BorderImageConfig::default();
        assert_eq!(bi.position, "behind");
        assert!(bi.is_background());
    }

    #[test]
    fn test_check_updates_config_and_aliases() {
        let cfg = Config::default();
        assert!(cfg.general.check_updates);

        let disabled: Config = toml::from_str("[general]\ncheck_updates = false\n").unwrap();
        assert!(!disabled.general.check_updates);

        let alias1: Config = toml::from_str("[general]\nupdate_check = false\n").unwrap();
        assert!(!alias1.general.check_updates);

        let alias2: Config = toml::from_str("[general]\nnotify_updates = false\n").unwrap();
        assert!(!alias2.general.check_updates);
    }

    #[test]
    fn test_write_if_not_exists_never_overwrites_user_config() {
        let dir =
            std::env::temp_dir().join(format!("rustfetch_cfg_preserve_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let cfg_path = dir.join("config.toml");

        let user_custom = "[general]\nseparator = \" -> \"\ntheme = \"catppuccin\"\n";
        assert!(Config::write_if_not_exists(&cfg_path, user_custom));
        assert_eq!(std::fs::read_to_string(&cfg_path).unwrap(), user_custom);

        // A subsequent call (e.g. after an update) MUST NOT overwrite the existing file
        assert!(!Config::write_if_not_exists(
            &cfg_path,
            &Config::to_default_toml()
        ));
        assert_eq!(std::fs::read_to_string(&cfg_path).unwrap(), user_custom);

        // And backup_if_exists creates config.toml.bak
        let bak = Config::backup_if_exists(&cfg_path).expect("backup should be created");
        assert_eq!(std::fs::read_to_string(&bak).unwrap(), user_custom);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_salvage_partial_toml_and_shorthand_modules() {
        // Shorthand string modules + border style string
        let shorthand = r#"
        [general]
        separator = " :: "
        border = "rounded"
        modules = ["os", "kernel", "cpu"]
        "#;
        // Note: modules at top level vs under general
        let shorthand_top = r#"
        modules = ["os", "kernel", "cpu"]
        [general]
        separator = " :: "
        border = "rounded"
        "#;
        let cfg = Config::from_toml(shorthand_top).unwrap();
        assert_eq!(cfg.general.separator, " :: ");
        assert!(cfg.general.border);
        assert_eq!(cfg.general.border_style.as_deref(), Some("rounded"));
        assert_eq!(cfg.modules.len(), 3);
        assert_eq!(cfg.modules[0].name, "os");
        let _ = shorthand;

        // Fault-tolerant salvage when one field has an incompatible type
        let partially_broken = r#"
        [general]
        separator = " >> "
        theme = "dracula"
        padding = "invalid_number_type"
        center = true

        [[modules]]
        name = "os"

        [[modules]]
        name = "cpu"
        "#;
        let salvaged = Config::from_toml(partially_broken).expect("should salvage valid fields");
        assert_eq!(salvaged.general.separator, " >> ");
        assert_eq!(salvaged.general.theme.as_deref(), Some("dracula"));
        assert!(salvaged.general.center);
        assert_eq!(salvaged.modules.len(), 2);
        assert_eq!(salvaged.modules[0].name, "os");
        assert_eq!(salvaged.modules[1].name, "cpu");
    }

    #[test]
    fn test_gpu_config_formats() {
        // 1. [gpu] top-level table
        let toml_gpu_top = r#"
        [gpu]
        type = "igpu"
        "#;
        let cfg1 = Config::from_toml(toml_gpu_top).unwrap();
        assert_eq!(cfg1.gpu.r#type, "igpu");
        let gpu_mod1 = cfg1.modules.iter().find(|m| m.name == "gpu").unwrap();
        assert_eq!(gpu_mod1.gpu_type.as_deref(), Some("igpu"));

        // 2. [modules.gpu] sub-table
        let toml_modules_gpu = r#"
        [modules.gpu]
        type = "all"
        "#;
        let cfg2 = Config::from_toml(toml_modules_gpu).unwrap();
        assert_eq!(cfg2.gpu.r#type, "all");
        let gpu_mod2 = cfg2.modules.iter().find(|m| m.name == "gpu").unwrap();
        assert_eq!(gpu_mod2.gpu_type.as_deref(), Some("all"));

        // 3. [[modules]] table with name = "gpu" and type = "igpu"
        let toml_item_type = r#"
        [[modules]]
        name = "gpu"
        type = "igpu"
        "#;
        let cfg3 = Config::from_toml(toml_item_type).unwrap();
        assert_eq!(cfg3.modules[0].name, "gpu");
        assert_eq!(cfg3.modules[0].gpu_type.as_deref(), Some("igpu"));

        // 4. [[modules]] table with name = "gpu" and gpu_type = "dgpu"
        let toml_item_gpu_type = r#"
        [[modules]]
        name = "gpu"
        gpu_type = "dgpu"
        "#;
        let cfg4 = Config::from_toml(toml_item_gpu_type).unwrap();
        assert_eq!(cfg4.modules[0].name, "gpu");
        assert_eq!(cfg4.modules[0].gpu_type.as_deref(), Some("dgpu"));

        // 5. Dedicated dgpu and igpu modules
        let toml_dedicated = r#"
        [[modules]]
        name = "igpu"
        label = "Internal Graphics"

        [[modules]]
        name = "dgpu"
        label = "Dedicated Graphics"
        "#;
        let cfg5 = Config::from_toml(toml_dedicated).unwrap();
        assert_eq!(cfg5.modules.len(), 2);
        assert_eq!(cfg5.modules[0].name, "igpu");
        assert_eq!(cfg5.modules[0].label.as_deref(), Some("Internal Graphics"));
        assert_eq!(cfg5.modules[1].name, "dgpu");
        assert_eq!(cfg5.modules[1].label.as_deref(), Some("Dedicated Graphics"));

        // 6. [[modules]] followed by [modules.gpu]
        let toml_modules_gpu_nested = r#"
        [[modules]]
        name = "os"

        [[modules]]
        name = "gpu"

        [modules.gpu]
        type = "igpu"
        "#;
        let cfg6 = Config::from_toml(toml_modules_gpu_nested).unwrap();
        assert_eq!(cfg6.modules.len(), 2);
        assert_eq!(cfg6.modules[0].name, "os");
        assert_eq!(cfg6.modules[1].name, "gpu");
        assert_eq!(cfg6.modules[1].gpu_type.as_deref(), Some("igpu"));

        // 7. filter and display aliases
        let toml_filter = r#"
        [[modules]]
        name = "gpu"
        filter = "all"
        "#;
        let cfg7 = Config::from_toml(toml_filter).unwrap();
        assert_eq!(cfg7.modules[0].gpu_type.as_deref(), Some("all"));

        let toml_display = r#"
        [[modules]]
        name = "gpu"
        display = "discrete"
        "#;
        let cfg8 = Config::from_toml(toml_display).unwrap();
        assert_eq!(cfg8.modules[0].gpu_type.as_deref(), Some("discrete"));

        // 8. Icons for dgpu and igpu
        assert_eq!(default_icon_for_module("gpu"), Some("󰢮"));
        assert_eq!(default_icon_for_module("dgpu"), Some("󰢮"));
        assert_eq!(default_icon_for_module("igpu"), Some("󰢮"));
    }

    #[test]
    fn test_clamp_values_from_toml() {
        let toml_str = r#"
        [general]
        padding = 999
        box_padding = 99
        border_image_width = 0
        border_image_height = 4096
        border_image_offset_x = -999999
        border_image_z_index = 999

        [general.colors]
        rows = 9

        [general.logo]
        image_width_cols = 9999

        [general.logo.three_d]
        size = 9.0
        speed = 0.001
        depth = 99.0
        width = 99999
        height = 0

        [general.logo.animation]
        fps = 999.0

        [[modules]]
        name = "memory"
        bar_width = 0
        "#;
        let cfg = Config::from_toml(toml_str).expect("valid TOML");
        assert_eq!(cfg.general.padding, 64);
        assert_eq!(cfg.general.box_padding, 16);
        assert_eq!(cfg.general.colors.rows, 2);
        assert_eq!(cfg.general.logo.image_width_cols, 256);
        assert_eq!(cfg.general.logo.three_d.size, 3.0);
        assert_eq!(cfg.general.logo.three_d.speed, 0.05);
        assert_eq!(cfg.general.logo.three_d.depth, 5.0);
        assert_eq!(cfg.general.logo.three_d.width, Some(512));
        assert_eq!(cfg.general.logo.three_d.height, Some(1));
        assert_eq!(cfg.general.logo.animation.fps, 120.0);
        assert_eq!(cfg.general.border_image_width, Some(1));
        assert_eq!(cfg.general.border_image_height, Some(512));
        assert_eq!(cfg.general.border_image_offset_x, Some(-10_000));
        assert_eq!(cfg.general.border_image_z_index, Some(100));
        assert_eq!(cfg.modules[0].bar_width, Some(1));
        let warnings = cfg.warnings();
        assert!(warnings.iter().any(|w| w.contains("general.padding")));
        assert!(warnings.iter().any(|w| w.contains("bar_width")));
    }

    #[test]
    fn test_non_finite_floats_fall_back_to_defaults() {
        let toml_str = r#"
        [general.logo.three_d]
        size = nan
        speed = inf
        "#;
        let cfg = Config::from_toml(toml_str).expect("valid TOML");
        assert_eq!(cfg.general.logo.three_d.size, 1.0);
        assert_eq!(cfg.general.logo.three_d.speed, 1.0);
        let warnings = cfg.warnings();
        assert!(warnings
            .iter()
            .any(|w| w.contains("general.logo.three_d.size") && w.contains("finite")));
        assert!(warnings
            .iter()
            .any(|w| w.contains("general.logo.three_d.speed") && w.contains("finite")));
    }

    #[test]
    fn test_shading_mode_and_bar_style_whitelists() {
        let toml_str = r#"
        [general.logo.three_d]
        shading_mode = "hologram"

        [[modules]]
        name = "memory"
        bar_style = "rainbow"

        [[modules]]
        name = "disk"
        bar_style = "line"
        "#;
        let cfg = Config::from_toml(toml_str).expect("valid TOML");
        assert_eq!(cfg.general.logo.three_d.shading_mode, "ascii");
        assert_eq!(cfg.modules[0].bar_style, None);
        assert_eq!(cfg.modules[1].bar_style.as_deref(), Some("line"));
        let warnings = cfg.warnings();
        assert!(warnings.iter().any(|w| w.contains("shading_mode")));
        assert!(warnings.iter().any(|w| w.contains("bar_style")));
    }

    #[test]
    fn test_border_chars_compact_string_and_table() {
        let compact = "[general]\nborder_chars = \"┌─┐│┘─└│├─┤\"\n";
        let cfg = Config::from_toml(compact).expect("compact border_chars must parse");
        let chars = cfg.general.border_chars.expect("border_chars set");
        assert_eq!(chars.top_left, "┌");
        assert_eq!(chars.top_right, "┐");
        assert_eq!(chars.bottom_right, "┘");
        assert_eq!(chars.divider_right, "┤");

        let table = r#"
        [general.border_chars]
        top_left = "X"
        top = "y"
        "#;
        let cfg_table = Config::from_toml(table).expect("table border_chars must parse");
        let chars_table = cfg_table.general.border_chars.expect("border_chars set");
        assert_eq!(chars_table.top_left, "X");
        assert_eq!(chars_table.divider_right, "┤");

        let invalid = "[general]\nborder_chars = \"┌─┐\"\n";
        assert!(Config::from_toml(invalid).is_err());
    }

    #[test]
    fn test_preset_layering_and_unknown_preset() {
        let toml_str = r#"
        preset = "card"

        [general]
        padding = 3
        separator = "!"
        "#;
        let cfg = Config::from_toml(toml_str).expect("valid TOML");
        assert!(cfg.general.border);
        assert!(cfg.general.center);
        assert_eq!(cfg.general.padding, 3);
        assert_eq!(cfg.general.separator, "!");
        assert!(cfg.modules.iter().any(|m| m.name == "host"));
        assert!(cfg.warnings().is_empty(), "warnings: {:?}", cfg.warnings());

        let unknown = "preset = \"neon-void\"\n";
        let cfg_unknown = Config::from_toml(unknown).expect("valid TOML");
        assert!(cfg_unknown.warnings().iter().any(|w| w.contains("preset")));
        assert_eq!(cfg_unknown.modules, Config::default().modules);
    }

    #[test]
    fn test_unknown_top_level_key_warns() {
        let toml_str = r#"
        unknown_thing = 1

        [general]
        padding = 1
        "#;
        let cfg = Config::from_toml(toml_str).expect("unknown keys must not fail parsing");
        let warnings = cfg.warnings();
        assert!(warnings
            .iter()
            .any(|w| w.contains("unknown top-level key 'unknown_thing'")));
    }

    #[test]
    fn test_unknown_module_and_theme_warnings() {
        let toml_str = r#"
        [general]
        theme = "neon-void"

        [[modules]]
        name = "not_a_module"

        [[modules]]
        name = "disk:/home"
        "#;
        let cfg = Config::from_toml(toml_str).expect("valid TOML");
        let warnings = cfg.warnings();
        assert!(warnings.iter().any(|w| w.contains("not_a_module")));
        assert!(warnings.iter().any(|w| w.contains("neon-void")));
        assert!(is_known_module_name("disk:/home"));
        assert!(is_known_theme_name("tokyonight"));
        assert!(is_known_theme_name("rose_pine"));
        assert!(!is_known_theme_name("definitely-not-a-theme"));
    }

    #[test]
    fn test_default_config_has_no_warnings_and_public_clamp() {
        let mut cfg = Config::default();
        assert!(cfg.warnings().is_empty(), "warnings: {:?}", cfg.warnings());
        cfg.general.padding = 1000;
        cfg.general.logo.three_d.shading_mode = "bogus".to_string();
        cfg.clamp();
        assert_eq!(cfg.general.padding, 64);
        assert_eq!(cfg.general.logo.three_d.shading_mode, "ascii");
        assert!(!cfg.warnings().is_empty());
    }
}
