# bevy_koto

---

[Koto][koto] scripting support for the [Bevy][bevy] game engine.

## Current State

This crate is serves as a proof of concept of integrating Koto with Bevy. 

You can see it in action by running the example application: 

`cargo run --release --example demo`

It's still early in development and hasn't been used outside of toy projects,
use at your own risk!

Your feedback is welcomed, please feel free to reach out via issues,
discussions, or the [Koto discord][discord].

## Features

- Modular plugins exposing various levels of integration.  
- Hot-reloading of Koto scripts using Bevy's asset system
- Mapping between Koto and Bevy entities
- Plugins for some useful Koto libraries like `color`, `geometry`, and `random`.
- Proof of concept plugins for scripted animation of 2d shapes.

## Supported Versions

**Bevy**: `v0.13`
**Koto**: `v0.14`

[bevy]: https://bevyengine.org
[discord]: https://discord.gg/JeV8RuK4CT
[koto]: https://koto.dev
