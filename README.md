# bevy_koto

---

[Koto][koto] scripting support for the [Bevy][bevy] game engine.

## Current State

This crate is serves as a proof of concept of integrating Koto with Bevy. 

You can see it in action by running the demo application: 

`cargo run --example demo`

Video of the demo in action:

[![Playing around with the bevy_koto demo](https://img.youtube.com/vi/EqgAEOucBP8/0.jpg)](https://www.youtube.com/watch?v=EqgAEOucBP8)

It's still early in development and hasn't been used outside of toy projects,
use at your own risk!

Your feedback is welcomed, please feel free to reach out via issues,
discussions, or the [Koto discord][discord].

## Features

- Modular plugins exposing various levels of integration.  
- Hot-reloading of Koto scripts using Bevy's asset system
- Mapping between Koto and Bevy entities
- Plugins for some useful Koto libraries like [`color`][koto_color], 
  [`geometry`][koto_geometry], and [`random`][koto_random].
- Proof of concept plugins for scripted animation of 2d shapes.

## Supported Versions

| `bevy_koto` | `bevy`  | `koto`  |
| ----------- | ------- | ------- |
| `v0.2`      | `v0.14` | `v0.14` |
| `v0.1`      | `v0.13` | `v0.14` |

[bevy]: https://bevyengine.org
[discord]: https://discord.gg/JeV8RuK4CT
[koto]: https://koto.dev
[koto_color]: https://koto.dev/docs/next/libs/color
[koto_geometry]: https://koto.dev/docs/next/libs/geometry
[koto_random]: https://koto.dev/docs/next/libs/random
