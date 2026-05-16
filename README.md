# st-click

A YAML-driven MIDI metronome / polyrhythm generator. Part of the
[st-suite](https://github.com/spencerharmon/st-suite) live performance rig.

`st-click` is a JACK MIDI client. It reads named rhythmic *sequences* from a
YAML config and emits MIDI Note-On messages on a JACK MIDI output port,
synchronized to the rest of the suite via [`st-sync`](../st-sync) (the
sample-accurate beat-frame protocol broadcast by
[`st-conductor`](../st-conductor)).

## Usage

```sh
st-click <sequence_name>
```

`<sequence_name>` is the top-level key in the YAML config (e.g. `backbeat`,
`4_against_7`). The JACK MIDI output port is named `st-click:midi_out` (or
similar — see JACK port listing).

Runtime requirements:

- A running JACK server.
- A running [`st-conductor`](../st-conductor) (provides JACK timebase and the
  `st-sync` TCP server on `127.0.0.1:6142`).

## Configuration

`st-click` looks for its config in this order; the first found wins:

1. `~/.config/st-tools/st-click.yaml`
2. `/etc/st-tools/st-click.yaml`

An example is shipped at `etc/st-tools/st-click.yaml`.

### YAML format

A config file is a YAML document whose top-level keys are *sequence names*.
Each sequence value may be either a **list of note entries** (legacy, implies
a 1-bar sequence) or a **mapping** with optional metadata:

```yaml
# Legacy list form — implicit `bars: 1`:
backbeat:
  - { note: "C-1", beat_value: 1.0, every: 2 }
  - { note: "D#/Eb-1", beat_value: 1.0, every: 2, skip: 1 }

# Mapping form — supports `bars: N` plus `notes:`:
rumba_clave_4bar:
  bars: 4
  notes:
    - { note: "C#/Db-1", beat_value: 16.0, offset: 0.00 }
    - { note: "C#/Db-1", beat_value: 16.0, offset: 1.00 }
```

Sequence-level fields (mapping form only):

| Field    | Type    | Required | Description                                              |
| -------- | ------- | -------- | -------------------------------------------------------- |
| `bars`   | integer | no (=1)  | How many bars the sequence spans before repeating.       |
| `notes`  | list    | yes      | The list of note entries (see below).                    |

Each note entry is a mapping with the following fields:

| Field        | Type         | Required | Description                                                                    |
| ------------ | ------------ | -------- | ------------------------------------------------------------------------------ |
| `note`       | string       | yes      | Note name, e.g. `"C4"`, `"D#/Eb1"`, `"C-1"`. See note-name list below.         |
| `beat_value` | float        | yes      | Note duration relative to a quarter note. `1.0` = quarter, `0.5` = eighth, `0.25` = sixteenth, `2.0` = half, `4.0` = whole. |
| `every`      | integer      | no (=1)  | Play on every Nth slot (a slot is one `beat_value`). `every: 2` halves the rate. |
| `skip`       | integer      | no (=0)  | Skip the first N slots before the pattern begins repeating.                    |
| `tuplet`     | integer      | no       | Convert `beat_value` into an *N*-tuplet over 2 of that value (e.g. `tuplet: 3` on a quarter gives a quarter-note triplet; `tuplet: 7` gives a septuplet over 2 quarters). |
| `offset`     | float        | no (=0)  | Shift every emitted hit by this many *quarter-note beats*. Signed (negative = anticipation / push-beat). Independent of `beat_value`, so composes with `tuplet`. Wraps modulo the sequence span. |

Note-name format: `<letter><accidental?><octave>` — examples: `C4`, `A0`,
`G9`, `C-1`. Sharps/flats are written together as `C#/Db4`, `D#/Eb1`, etc.
See `src/note_utils.rs` for the complete table (range `C-1` through `G9`,
matching the standard MIDI note range).

### Example

```yaml
---
backbeat:
  - note: "C-1"          # downbeat: every other quarter starting on beat 1
    every: 2
    beat_value: 1.0
  - note: "D#/Eb1"       # backbeat: every other quarter starting on beat 2
    every: 2
    skip: 1
    beat_value: 1.0

4_against_7:
  - note: "C-1"          # 4 quarter-note clicks per bar
    every: 1
    beat_value: 1.0
  - note: "D#/Eb1"       # 7 evenly spaced clicks against the same bar
    every: 1
    beat_value: 2.0
    tuplet: 7
```

## Status

Early / experimental. No README on the upstream repo before this one; the
YAML schema is whatever `src/config.rs` accepts.
