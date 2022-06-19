use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};



fn main() -> anyhow::Result<()> {
    // Find default output host
    let host = cpal::default_host();

    // assign output device from host
    let device = host
        .default_output_device()
        .expect("failed to find output device");
    println!("Output device: {:?}", device.name()?); // print result

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);

    run::<f32>(&device, &config.into())?;
    // Return result
    // TODO: read up on anyhow crate
    Ok(())
}

pub fn run<T>(device: &cpal::Device, config: &cpal::StreamConfig) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let keynumber: usize = 60; // associated midi keynumber -> 60 == 'C4'
    // setting stage for midi callback to take a number to generate a tone
    // todo: use channels

    let freq = { 440.0 * (2.0 as f32).powf((keynumber as f32 - 69.0) / 12.0) };

    let mut sample_clock = 0f32;
    let mut next_value = move || {
        sample_clock = (sample_clock + 1.0) % sample_rate;

        // Sine calc
        // (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()\

        let square = 4.0 * (freq * sample_clock / sample_rate).floor()
            - 2.0 * (2.0 * freq * sample_clock / sample_rate).floor()
            + 1.0;
        square * 0.5 // Currently half amplitude
    };
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            write_data(data, channels, &mut next_value)
        },
        err_fn,
    )?;
    stream.play()?;

    std::thread::sleep(std::time::Duration::from_millis(1000));
    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let value: T = cpal::Sample::from::<f32>(&next_sample());
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}
