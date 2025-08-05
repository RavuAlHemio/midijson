mod midi;


use std::fs::File;
use std::io::{BufReader, Write};
use std::path::PathBuf;

use clap::{Args, Parser};

use crate::midi::model::StandardMidiFile;
use crate::midi::rw_asm::{read_asm, write_asm};
use crate::midi::rw_smf::{read_smf, write_smf};


#[derive(Parser)]
enum OptsMode {
    #[command(name = "midi2json")] Midi2Json(MidiToJsonOpts),
    #[command(name = "json2midi")] Json2Midi(JsonToMidiOpts),
    #[command(name = "disas")] Disassemble(DisassembleOpts),
    #[command(name = "asm")] Assemble(AssembleOpts),
}

#[derive(Args)]
struct MidiToJsonOpts {
    pub input_midi_file: PathBuf,
    pub output_json_file: PathBuf,
}

#[derive(Args)]
struct JsonToMidiOpts {
    pub input_json_file: PathBuf,
    pub output_midi_file: PathBuf,
}

#[derive(Args)]
struct AssembleOpts {
    pub input_asm_file: PathBuf,
    pub output_midi_file: PathBuf,
}

#[derive(Args)]
struct DisassembleOpts {
    pub input_midi_file: PathBuf,
    pub output_asm_file: PathBuf,
}


fn main() {
    let mode = OptsMode::parse();
    match mode {
        OptsMode::Midi2Json(m2j) => {
            let mut input_midi = File::open(&m2j.input_midi_file)
                .expect("failed to open input MIDI file");
            let midi = read_smf(&mut input_midi)
                .expect("failed to read input MIDI file");
            let json = serde_json::to_string_pretty(&midi)
                .expect("failed to serialize MIDI to JSON");
            std::fs::write(&m2j.output_json_file, &json)
                .expect("failed to write output JSON file");
        },
        OptsMode::Json2Midi(j2m) => {
            let input_string = std::fs::read_to_string(&j2m.input_json_file)
                .expect("failed to read input JSON file");
            let input_midi: StandardMidiFile = serde_json::from_str(&input_string)
                .expect("failed to parse input JSON");

            {
                let mut output = File::create(&j2m.output_midi_file)
                    .expect("failed to create output MIDI file");
                write_smf(&input_midi, &mut output)
                    .expect("failed to write output MIDI file");
                output.flush()
                    .expect("failed to flush output MIDI file");
            }
        },
        OptsMode::Assemble(asm) => {
            let input_asm = File::open(&asm.input_asm_file)
                .expect("failed to open input assembly file");
            let mut buf_input_asm = BufReader::new(input_asm);
            let midi = read_asm(&mut buf_input_asm)
                .expect("failed to read input assembly file");

            {
                let mut output = File::create(&asm.output_midi_file)
                    .expect("failed to create output MIDI file");
                write_smf(&midi, &mut output)
                    .expect("failed to write output MIDI file");
                output.flush()
                    .expect("failed to flush output MIDI file");
            }
        },
        OptsMode::Disassemble(dasm) => {
            let mut input_midi = File::open(&dasm.input_midi_file)
                .expect("failed to open input MIDI file");
            let midi = read_smf(&mut input_midi)
                .expect("failed to read input MIDI file");

            {
                let mut output = File::create(&dasm.output_asm_file)
                    .expect("failed to create output assembly file");
                write_asm(&midi, &mut output)
                    .expect("failed to write output assembly file");
                output.flush()
                    .expect("failed to flush output assembly file");
            }
        },
    }
}
