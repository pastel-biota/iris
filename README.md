# 🖼️ pastel-biota/iris

My self-hosted photo management infrastructure!

> [!NOTE]
> This repository does not have any frontend - the web interface is served by [📚 Iridescence](https://github.com/pastel-biota/iridescence).

## Features

- 💽 Is a headless HTTP REST API service without any frontend application.
- 📝 Easy to control / export - Filesystem + JSON file based state management
- 🔐 Reading is public, but writing requires the auth
- ↔️ Resizes into preconfigured various resolutions.
  - 📨 It's queued! So if I update many photo at once the resource won't die (hopefully [^just-releasing-now])
    - (Currently has an issue of hogging tons of RAM if the app receives too many photos, so the image processing does be fine but the ingestion isn't. It's being pointless a bit I need to fix this)
- 🔍 Parses the EXIF metadata and a HEX color close to the photo. Easy to provide rich experience with the photo
- 🧻 Post-processing of the parsed metadata. (Currently it just has removing the geolocation data though.)
- 🦀 It's rusty
  - A huge, HUGE shoutout to [various crates](/Cargo.toml) and Rust teams. 95% of my app's value is being delivered from these not even me

[^just-releasing-now]: I'm writing this while the final image for the release is being build. Real-world scenario for this app is the first time, so I'm a little bit nervous about this

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
      01-***.json
      01-***/
         01-***-icon.webp
         01-***-thumbnail.webp
         ...
  ```
- Photo's information is recorded as JSON!

This allows the state to be easily modifiable and trackable, and nothing is locked behind the weird binary arbitrary thingamajig.
But every operation includes text ser/de-ing, I/O operation, and so on... not the fastest. But hey this is for my personal use, so well this works for me!

