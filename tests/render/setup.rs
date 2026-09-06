use planet_crafter_engine::testing::device_type_rank;
use vulkano::device::physical::PhysicalDeviceType;

#[test]
fn device_type_rank_prefers_discrete_gpu() {
    let discrete = device_type_rank(PhysicalDeviceType::DiscreteGpu);
    let integrated = device_type_rank(PhysicalDeviceType::IntegratedGpu);
    let virtual_gpu = device_type_rank(PhysicalDeviceType::VirtualGpu);
    let cpu = device_type_rank(PhysicalDeviceType::Cpu);
    let other = device_type_rank(PhysicalDeviceType::Other);
    assert!(discrete < integrated);
    assert!(integrated < virtual_gpu);
    assert!(virtual_gpu < cpu);
    assert!(cpu < other);
}
