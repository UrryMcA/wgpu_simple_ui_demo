use anyhow::Result;
use wgpu::{Device, Queue, Instance, Adapter, MemoryBudgetThresholds, ExperimentalFeatures};

pub struct GraphicsContext {
    pub instance: Instance,
    pub device: Device,
    pub queue: Queue,
    pub adapter: Adapter,
}

impl GraphicsContext {
    pub async fn new() -> Result<Self> {
        let instance = Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions::default(),
            display: None,
            memory_budget_thresholds: MemoryBudgetThresholds::default(),
        });
        
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find adapter");
        
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                    trace: wgpu::Trace::Off,
                    experimental_features: ExperimentalFeatures::disabled(),
                },
            )
            .await?;
        
        Ok(Self { instance, device, queue, adapter })
    }
}