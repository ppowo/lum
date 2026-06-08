# Radio Playback Drops Pause and Resume

Status: Accepted

## Context

`lum radio` used to model playback as three states: stopped, playing, and paused. Because playback is delegated to a detached `ffplay` process, pause was implemented by stopping `ffplay` and recording the station as paused. Resume started a new `ffplay` process for that remembered station.

That interface added a paused flag to `radio-player.json`, two CLI commands, two dispatch paths, and status output for a state that behaved like stop-then-play.

## Decision

Remove pause and resume from the `lum radio` interface.

The supported `lum radio` commands are:

- `lum radio` / `lum radio list` to list stations
- `lum radio <code>` to play a station, replacing any current station
- `lum radio status` to report remembered playback state
- `lum radio stop` to stop playback and clear state

The stored radio state contains only the remembered `ffplay` process identity and station metadata. It does not contain a paused flag.

## Consequences

The playback lifecycle is two-state: stopped or playing.

`status` prints `playing <code> <description>` only when the remembered process is still an alive `ffplay` process. Otherwise it clears stale state and prints `stopped`.

`pause` and `resume` are no longer reserved radio commands. They are parsed like any other station code and will fail as unknown stations unless a station with that code exists.

If future requirements need pause-like behavior, prefer an explicit new ADR before widening the radio interface again.
