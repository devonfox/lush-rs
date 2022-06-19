
use cpal::traits::{DeviceTrait, HostTrait};

fn main() -> anyhow::Result<()> {
    // Find default output host
    let host = cpal::default_host();

    // assign output device from host
    let device = host.default_output_device().expect("failed to find output device");
    println!("Output device: {:?}", device.name()?); // print result

    // Return result
    // TODO: read up on anyhow crate
    Ok(())
}

