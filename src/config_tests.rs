use crate::{
    Lighting2dSettings,
    config::{LightingBackend2d, LightingCompositeMode2d, ShadowFiltering2d},
};

#[test]
fn lighting_settings_defaults_are_reasonable() {
    let settings = Lighting2dSettings::default();

    assert!(settings.lighting_enabled);
    assert!(settings.lightmap_scale > 0.0);
    assert!(settings.normal_map_scale > 0.0);
    assert!(settings.enable_occlusion);
    assert!(settings.enable_normal_maps);
    assert!(settings.enable_emissive);
    assert_eq!(settings.backend, LightingBackend2d::ScreenSpace);
    assert_eq!(settings.shadow_filter, ShadowFiltering2d::Hard);
    assert_eq!(settings.composite_mode, LightingCompositeMode2d::Multiply);
}

#[test]
fn fast_unshadowed_preset_disables_occlusion() {
    let settings = Lighting2dSettings::fast_unshadowed();

    assert!(settings.lighting_enabled);
    assert!(!settings.enable_occlusion);
    assert_eq!(settings.shadow_filter, ShadowFiltering2d::Hard);
}

#[test]
fn showcase_soft_preset_enables_soft_shadows() {
    let settings = Lighting2dSettings::showcase_soft();

    assert!(settings.enable_occlusion);
    assert_eq!(settings.shadow_filter, ShadowFiltering2d::Soft);
}
