use bevy::{
    asset::{Asset, RenderAssetUsages},
    image::Image,
    mesh::MeshVertexBufferLayoutRef,
    prelude::*,
    reflect::TypePath,
    render::render_resource::{
        AsBindGroup, BlendComponent, BlendFactor, BlendOperation, BlendState, Extent3d,
        RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError, TextureDimension,
        TextureFormat,
    },
    shader::ShaderRef,
    sprite_render::{AlphaMode2d, Material2d},
};

use crate::{
    AMBIENT_OVERLAY_SHADER_HANDLE, LIGHT_BLUR_SHADER_HANDLE, LIGHT_COMPOSITE_SHADER_HANDLE,
    LIGHT_RENDER_SHADER_HANDLE, RECEIVER_LIGHTING_SHADER_HANDLE,
};

pub(crate) const MAX_OCCLUDER_SEGMENTS: usize = 48;
pub(crate) const MAX_RECEIVER_LIGHTS: usize = 8;
pub(crate) const MAX_RECEIVER_COOKIE_TEXTURES: usize = 4;

#[derive(Resource, Default)]
pub(crate) struct Lighting2dInternalAssets {
    pub quad_mesh: Handle<Mesh>,
    pub white_image: Handle<Image>,
    pub flat_normal_image: Handle<Image>,
    pub white_mask_image: Handle<Image>,
}

impl Lighting2dInternalAssets {
    #[must_use]
    pub(crate) fn make_white_image() -> Image {
        Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[255, 255, 255, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        )
    }

    #[must_use]
    pub(crate) fn make_flat_normal_image() -> Image {
        Image::new_fill(
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[128, 128, 255, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::default(),
        )
    }
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub(crate) struct SegmentUniform {
    pub segment: Vec4,
    pub transmission: Vec4,
    pub metadata: UVec4,
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub(crate) struct AmbientOverlayUniform {
    pub tint: Vec4,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub(crate) struct AmbientOverlayMaterial {
    #[uniform(0)]
    pub uniform: AmbientOverlayUniform,
}

impl Material2d for AmbientOverlayMaterial {
    fn fragment_shader() -> ShaderRef {
        AMBIENT_OVERLAY_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::sprite_render::Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(target) = descriptor
            .fragment
            .as_mut()
            .and_then(|fragment| fragment.targets.get_mut(0))
            .and_then(Option::as_mut)
        {
            target.blend = Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::Dst,
                    dst_factor: BlendFactor::Zero,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::Zero,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub(crate) struct LightMaterialUniform {
    pub color: Vec4,
    pub position_and_kind: Vec4,
    pub radius_and_angles: Vec4,
    pub direction_and_flags: Vec4,
    pub texture_size_and_rotation: Vec4,
    pub source_params: Vec4,
    pub metadata: UVec4,
    pub segments: [SegmentUniform; MAX_OCCLUDER_SEGMENTS],
}

impl Default for LightMaterialUniform {
    fn default() -> Self {
        Self {
            color: Vec4::ZERO,
            position_and_kind: Vec4::ZERO,
            radius_and_angles: Vec4::ZERO,
            direction_and_flags: Vec4::ZERO,
            texture_size_and_rotation: Vec4::ZERO,
            source_params: Vec4::ZERO,
            metadata: UVec4::ZERO,
            segments: [SegmentUniform::default(); MAX_OCCLUDER_SEGMENTS],
        }
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub(crate) struct LightRenderMaterial {
    #[uniform(0)]
    pub uniform: LightMaterialUniform,
    #[texture(1)]
    #[sampler(2)]
    pub cookie_texture: Option<Handle<Image>>,
}

impl Material2d for LightRenderMaterial {
    fn fragment_shader() -> ShaderRef {
        LIGHT_RENDER_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::sprite_render::Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(target) = descriptor
            .fragment
            .as_mut()
            .and_then(|fragment| fragment.targets.get_mut(0))
            .and_then(Option::as_mut)
        {
            target.blend = Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub(crate) struct ReceiverLightUniform {
    pub color: Vec4,
    pub position_and_kind: Vec4,
    pub radius_and_angles: Vec4,
    pub direction_and_source: Vec4,
    pub cookie_size_and_rotation: Vec4,
    pub metadata: UVec4,
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub(crate) struct ReceiverLightingUniform {
    pub base_color: Vec4,
    pub emissive_color: Vec4,
    pub uv_rect: Vec4,
    pub params: Vec4,
    pub flags: Vec4,
    pub lights: [ReceiverLightUniform; MAX_RECEIVER_LIGHTS],
    pub segments: [SegmentUniform; MAX_OCCLUDER_SEGMENTS],
}

impl Default for ReceiverLightingUniform {
    fn default() -> Self {
        Self {
            base_color: Vec4::ZERO,
            emissive_color: Vec4::ZERO,
            uv_rect: Vec4::ZERO,
            params: Vec4::ZERO,
            flags: Vec4::ZERO,
            lights: [ReceiverLightUniform::default(); MAX_RECEIVER_LIGHTS],
            segments: [SegmentUniform::default(); MAX_OCCLUDER_SEGMENTS],
        }
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub(crate) struct ReceiverLightingMaterial {
    #[uniform(0)]
    pub uniform: ReceiverLightingUniform,
    #[texture(1)]
    #[sampler(2)]
    pub diffuse_texture: Option<Handle<Image>>,
    #[texture(3)]
    #[sampler(4)]
    pub normal_texture: Option<Handle<Image>>,
    #[texture(5)]
    #[sampler(6)]
    pub emissive_mask: Option<Handle<Image>>,
    #[texture(7)]
    #[sampler(8)]
    pub cookie_texture_0: Option<Handle<Image>>,
    #[texture(9)]
    #[sampler(10)]
    pub cookie_texture_1: Option<Handle<Image>>,
    #[texture(11)]
    #[sampler(12)]
    pub cookie_texture_2: Option<Handle<Image>>,
    #[texture(13)]
    #[sampler(14)]
    pub cookie_texture_3: Option<Handle<Image>>,
}

impl Material2d for ReceiverLightingMaterial {
    fn fragment_shader() -> ShaderRef {
        RECEIVER_LIGHTING_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::sprite_render::Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(target) = descriptor
            .fragment
            .as_mut()
            .and_then(|fragment| fragment.targets.get_mut(0))
            .and_then(Option::as_mut)
        {
            target.blend = Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            });
        }

        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, ShaderType)]
pub(crate) struct LightBlurUniform {
    pub direction_and_radius: Vec4,
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub(crate) struct LightBlurMaterial {
    #[uniform(0)]
    pub uniform: LightBlurUniform,
    #[texture(1)]
    #[sampler(2)]
    pub source_texture: Option<Handle<Image>>,
}

impl Material2d for LightBlurMaterial {
    fn fragment_shader() -> ShaderRef {
        LIGHT_BLUR_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Opaque
    }
}

#[derive(Asset, AsBindGroup, TypePath, Debug, Clone, Default)]
pub(crate) struct LightCompositeMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub source_texture: Option<Handle<Image>>,
}

impl Material2d for LightCompositeMaterial {
    fn fragment_shader() -> ShaderRef {
        LIGHT_COMPOSITE_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }

    fn specialize(
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::sprite_render::Material2dKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        if let Some(target) = descriptor
            .fragment
            .as_mut()
            .and_then(|fragment| fragment.targets.get_mut(0))
            .and_then(Option::as_mut)
        {
            target.blend = Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            });
        }

        Ok(())
    }
}
