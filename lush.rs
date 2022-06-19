use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rand::Rng;
use std::io::stdin;
use std::sync::mpsc::*;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::spawn;

use lush_rs::Key;

fn main() -> anyhow::Result<()> {
    // Find default output host
    let host = cpal::default_host();

    // assign output device from host
    let device = host
        .default_output_device()
        .expect("failed to find output device");
    println!("Output device: {:?}", device.name()?); // print result

    let end_chan: (Sender<()>, Receiver<()>) = channel(); // end channel

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);

    let key: usize = rand::thread_rng().gen_range(20..84); // associated midi keynumber -> 60 == 'C4'
                                                           // setting stage for midi callback to take a number to generate a tone
                                                           // todo: use channels
    let note = Key {
        state: true,
        keynumber: key,
    };

    println!("{:?}", note);

    // Sending single note to thread to play until keypress is accepted in CLI
    let run_thread = spawn(move || run::<f32>(&device, &config.into(), note, end_chan.1));

    let mut input = String::new();
    let stdin = stdin();
    input.clear();
    match stdin.read_line(&mut input) {
        Ok(_) => println!("Ending program..."),
        Err(err) => println!("Error: {}", err),
    }; // wait for next enter key press to end program

    // Send end note
    let _ = end_chan.0.send(());

    // Join thread
    match run_thread.join() {
        Ok(_) => (),
        Err(error) => println!("Error: {:?}", error),
    };
    // Return result
    // TODO: read up on anyhow crate
    Ok(())
}

pub fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    note: Key,
    rx: Receiver<()>,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    println!("Sample Rate: {}", sample_rate);
    let channels = config.channels as usize;
    println!("Channels: {}", channels);

    let freq = { 440.0 * (2.0_f32).powf((note.keynumber as f32 - 69.0) / 12.0) };

    let mut sample_clock = 0f32;
    let mut next_value = move || {
        sample_clock += 1.0;

        // Sine calc
        // (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()\

        let square = 4.0 * (freq * sample_clock / sample_rate).floor()
            - 2.0 * (2.0 * freq * sample_clock / sample_rate).floor()
            + 1.0;
        square * 0.55 // Currently half amplitude
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
    let _ = rx.recv();

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
