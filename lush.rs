use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use lush_rs::Key;
use midir::{Ignore, MidiInput, MidiInputPort};
use std::error::Error;
use std::io::{stdin, stdout, Write};
use std::sync::mpsc::*;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::spawn;
use std::time::Duration;
use wmidi::MidiMessage;
use wmidi::MidiMessage::*;
use wmidi::Note;

fn main() -> anyhow::Result<()> {
    // Find default output host
    let host = cpal::default_host();

    // assign output device from host
    let device = host
        .default_output_device()
        .expect("failed to find output device");
    println!("Output device: {:?}", device.name()?); // print result

    let end_chan: (Sender<()>, Receiver<()>) = channel(); // end channel
    let note_chan: (Sender<Note>, Receiver<Note>) = channel(); // note channel

    let config = device.default_output_config().unwrap();
    println!("Default output config: {:?}", config);

    let mut midi_in = MidiInput::new("midir reading input")?;

    let in_ports = midi_in.ports();
    let in_port = match in_ports.len() {
        0 => {
            println!("Error: No input connection found. Press enter to quit.");
            Err("closing connections.").unwrap()
        }
        1 => {
            println!(
                "Choosing the only available input port: {}",
                midi_in.port_name(&in_ports[0]).unwrap()
            );
            &in_ports[0]
        }
        _ => {
            println!("\nAvailable input ports:");
            for (i, p) in in_ports.iter().enumerate() {
                println!("{}: {}", i, midi_in.port_name(p).unwrap());
            }
            print!("Please select input port: ");
            stdout().flush()?;
            let mut input = String::new();
            stdin().read_line(&mut input)?;
            in_ports
                .get(input.trim().parse::<usize>()?) // investigate
                .ok_or("invalid input port selected")
                .unwrap()
        }
    };
    midi_in.ignore(Ignore::None);
    let in_port_name = midi_in.port_name(in_port)?;
    let in_port = in_port.clone();

    let _read_thread = spawn(
        move || match read(&in_port_name, in_port, midi_in, note_chan.0) {
            Ok(_) => (),
            Err(err) => println!("Error: {}", err),
        },
    );
   
    // let note = Key {
    //     state: true,
    //     keynumber: key,
    // };
     let mut notes: Vec<Key> = Vec::with_capacity(128);

    for i in 0..128 {
        let keys = Key {
            state: false,
            keynumber: i,
            sample_clock: 0.0,
        };
        notes.push(keys);
    }

    // println!("{:?}", notes);
    let note = note_chan.1.recv()?;
    // println!("{:?}", note as usize);
    notes[note as usize].state = true;
    
    // Sending single note to thread to play until keypress is accepted in CLI
    let run_thread = spawn(move || run::<f32>(&device, &config.into(), notes, end_chan.1));

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
    mut notes: Vec<Key>,
    rx: Receiver<()>,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    println!("Sample Rate: {}", sample_rate);
    let channels = config.channels as usize;
    println!("Channels: {}", channels);
    let mut current = 0_usize;
    for note in &notes {
        if note.state {
            current = note.keynumber;
            break; // here we need assign the note to a different state
                   // and also add a note on or off to the setting
        }
    }

    let freq = { 440.0 * (2.0_f32).powf((current as f32 - 69.0) / 12.0) };

    // let freq = note.to_freq_f32();

    // let mut sample_clock = 0f32;
    let mut next_value = move || {
        notes[current].sample_clock += 1.0;

        // Sine calc
        // (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sample_rate).sin()\

        let square = 4.0 * (freq * notes[current].sample_clock / sample_rate).floor()
            - 2.0 * (2.0 * freq * notes[current].sample_clock / sample_rate).floor()
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

/// Reads midi input and sends the midi notes parsed from midi messages through
/// channels to our output midi function operating in a separate thread.
pub fn read(
    in_port_name: &str,
    in_port: MidiInputPort,
    midi_in: MidiInput,
    tx: Sender<Note>,
) -> Result<(), Box<dyn Error>> {
    println!("\nOpening input connection");
    let _input_connection = midi_in.connect(
        &in_port,
        "midir-read-input",
        move |_, message, _| {
            //println!("{:?} (len = {})", message, message.len());
            let message = MidiMessage::try_from(message).unwrap(); //unwrapping message slice
            if let NoteOn(_, note, _) = message {
                
                let _ = tx.send(note); // sending note value through channel
            }
        },
        (),
    )?;

    println!(
        "Input connection open, reading input from '{}' (press enter to stop input): ",
        in_port_name
    );

    loop {
        // fix ending condition later once note structure is set up
        std::thread::sleep(Duration::from_millis(50));
    }
    // _input_connection.close();
    // println!("Closing input connection");
    Ok(())
}
