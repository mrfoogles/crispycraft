use wgpu::*;
use super::lib::types::BindGroupSource;
use super::lib::util::fast_buffer;

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

pub struct CameraData {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    transform: [[f32; 4]; 4]
}

impl CameraData {
    pub fn build_transform(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * proj * view
    }
    pub fn uniform(&self) -> CameraUniform {
        CameraUniform {
            transform: self.build_transform().into()
        }
    }
}

impl BindGroupSource<Buffer> for CameraData {
    fn bind_group_layout(&self, device: &Device) -> BindGroupLayout {
        device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: None,
            entries: &[
            // Camera buffer
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            ],
        })
    }
    
    fn bind_group(
        &self,
        device: &Device,
        _queue: &Queue,
        layout: &BindGroupLayout,
    ) -> (BindGroup, Buffer) {
        let buffer = fast_buffer(
            device,
            &[self.uniform()],
            BufferUsages::COPY_DST | BufferUsages::UNIFORM,
        );
        let group = device.create_bind_group(&BindGroupDescriptor {
            label: None,
            layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        
        (group, buffer)
    }
    
    // Optional to implement
    fn update_bind_group(&self, data: &Buffer, queue: &Queue) {
        queue.write_buffer(data, 0, bytemuck::cast_slice(&[self.uniform()]));
    }
}