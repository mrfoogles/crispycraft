pub const VERT_ENTRY_POINT: &str = "vs_main";
pub const FRAG_ENTRY_POINT: &str = "fs_main";

pub mod types {
    use super::util;
    use wgpu::*;

    pub type Index = u16;
    pub const INDEX_FORMAT: IndexFormat = IndexFormat::Uint16;

    /// The data necessary to make a GPUMesh
    /// Meshes are generated on the CPU, then uploaded to the GPU
    /// ```rust
    /// let world = terrain::TerrainState::new();
    /// let gpu_mesh = world.make_mesh(1.);
    /// // In render()
    /// draw_mesh(&gpu_mesh, ...);
    /// ```
    #[derive(Debug)]
    pub struct CPUMesh<V: Clone + bytemuck::Pod> {
        pub verts: Vec<V>,
        pub indxs: Vec<Index>,
    }
    impl<V: Clone + bytemuck::Pod> CPUMesh<V> {
        pub fn upload_sized(&self, device: &Device, max_verts: u32, max_indxs: u32) -> GPUMesh<V> {
            // Pad verts with empty vertices if necessary (you can't make a buffer bigger than the data you put in it)
            let mut ext_verts = self.verts.clone();
            if max_verts > self.verts.len() as u32 {
                ext_verts.extend(
                    std::iter::repeat(V::zeroed()).take(max_verts as usize - ext_verts.len()),
                );
            }

            // Pad indxs as well
            let mut ext_indxs = self.indxs.clone();
            if max_indxs > self.indxs.len() as u32 {
                ext_indxs.extend(std::iter::repeat(0).take(max_indxs as usize - ext_indxs.len()));
            }

            let vertbuf = util::fast_buffer(
                device,
                &ext_verts,
                BufferUsages::VERTEX | BufferUsages::COPY_DST,
            );
            let indxbuf = util::fast_buffer(
                device,
                &ext_indxs,
                BufferUsages::INDEX | BufferUsages::COPY_DST,
            );

            GPUMesh {
                vertbuf,
                max_verts,
                indxbuf,
                num_indxs: self.indxs.len() as u32,
                max_indxs,

                _phantom_vert_data: core::marker::PhantomData,
            }
        }
        pub fn upload(&self, device: &Device) -> GPUMesh<V> {
            self.upload_sized(device, self.verts.len() as u32, self.indxs.len() as u32)
        }

        pub fn update_gpu_mesh(&self, gpu_mesh: &mut GPUMesh<V>, queue: &mut Queue) {
            // TODO: using the max_verts & max_indxs fields, check if this CPUMesh has too many vertices or indices

            queue.write_buffer(&gpu_mesh.vertbuf, 0, bytemuck::cast_slice(&self.verts));

            queue.write_buffer(&gpu_mesh.indxbuf, 0, bytemuck::cast_slice(&self.indxs));
            gpu_mesh.num_indxs = self.indxs.len() as u32;
        }
    }

    /// A reference to the GPU Buffers that hold a mesh
    /// Necessary to render a mesh
    /// ```rust
    /// let gpu_mesh = CPUMesh { verts: todo!(), indxs: todo!() }.upload();
    /// ```
    pub struct GPUMesh<V> {
        pub vertbuf: Buffer,
        pub max_verts: u32,

        pub indxbuf: Buffer,
        pub num_indxs: u32,
        pub max_indxs: u32,

        _phantom_vert_data: core::marker::PhantomData<V>,
    }

    pub trait BindGroupSource<DATA> {
        fn bind_group_layout(&self, device: &Device) -> BindGroupLayout;
        fn bind_group(
            &self,
            device: &Device,
            queue: &Queue,
            layout: &BindGroupLayout,
        ) -> (BindGroup, DATA);

        #[allow(unused_variables)]
        fn update_bind_group(&self, data: &DATA, queue: &Queue) {}
    }
}

pub mod util {
    use super::types::*;
    use wgpu::util::DeviceExt;
    use wgpu::*;

    pub fn fast_buffer<T: bytemuck::Pod>(
        device: &Device,
        data: &[T],
        usage: BufferUsages,
    ) -> Buffer {
        return device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("{:?} Buffer", usage)),
            contents: bytemuck::cast_slice(data),
            usage,
        });
    }

    pub fn draw_mesh<'a, 'b: 'a, V>(
        pass: &mut RenderPass<'a>,
        mesh: &'b GPUMesh<V>,
        instances: u32,
    ) {
        pass.set_vertex_buffer(0, mesh.vertbuf.slice(..));
        pass.set_index_buffer(mesh.indxbuf.slice(..), INDEX_FORMAT);

        pass.draw_indexed(0..mesh.num_indxs, 0, 0..instances);
    }

    pub async fn setup_wgpu(
        window: &winit::window::Window,
        power_pref: PowerPreference,
        device_desc: DeviceDescriptor<'_>,
        present_mode: PresentMode,
    ) -> (Surface, SurfaceConfiguration, Device, Queue) {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power_pref,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &device_desc,
                None, // Trace path
            )
            .await
            .unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: present_mode,
        };
        surface.configure(&device, &config);

        (surface, config, device, queue)
    }
}
