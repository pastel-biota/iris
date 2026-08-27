# 🖼️ pastel-biota/iris

My self-hosted photo management infrastructure!

> [!NOTE]
> This repository does not have any frontend - the web interface is served by [📚 Iridescence](https://github.com/pastel-biota/iridescence).

## Features

- 💽 Is a headless HTTP REST API service without any frontend application.
- 📝 Easy to control / export - Filesystem + JSON file based state management (with managed SQLite index for faster querying)
- 🔐 Reading is public, but writing requires the auth
- ↔️ Resizes into preconfigured various resolutions.
  - 📨 It's queued with resource gatekeeping! The resource won't suffer if many photos were uploaded at once
- 🔍 Parses the EXIF metadata and a HEX color close to the photo. Easy to provide rich experience with the photo
- 🧻 Post-processing of the parsed metadata. (Currently it just has removing the geolocation data though.)
- 🦀 It's rusty
  - A huge, HUGE shoutout to [various crates](/Cargo.toml) and Rust teams. 95% of my app's value is being delivered from these not even me

## How does this work? / Why though?

### Iris itself is just a service with HTTP REST API

Iris is intentionally being just a headless REST API service, not being coupled by any other interface.

This allows me to create various interface! I might be able to create some CLI frontend, or I can access/integrate with other system.

### Everything is based on the readable / easy-to-handle file format

This application records/serves the photo files in the local directory. 

- Uses full-on file system to store the resources, like:
  ```
  2026/
    03/
      202603-***-meta.json
      202603-***/
         202603-***-icon.webp
         202603-***-thumbnail.webp
         ...
  ```
- Photo's information is recorded as JSON!

This allows the state to be easily modifiable and trackable, and nothing is locked behind the weird binary arbitrary thingamajig.
Modify the JSON[^need-to-restart-iris] and the Iris' photo state is updated.

To improve the speed, the JSON payload is also saved to the SQLite by Iris as an index, and Iris will use that to provide faster (and much more sane) filtering and builk returning.
The JSON file is still canonical data, and SQLite index content is updated once the JSON file is modified[^need-to-restart-iris].

[^need-to-restart-iris]: Actually the Iris needs to be restarted to reflect the JSON file change. The file watcher will be added later

