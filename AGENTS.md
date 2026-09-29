# cocoa-ui

A safe, imperative Rust API over `AppKit` and `UIKit`, built on `objc2`.

## The layering rule

`cocoa-ui` wraps `AppKit`/`UIKit` faithfully and contains **no
SwiftUI-parity logic**. APIs are named after the platform API they wrap and
report the platform's own answers — its metrics, its defaults, its
geometry.

Parity and behaviour alignment live in `waterui-apple`: insets, frame
offsets, default sizes, measurement adjustments, toolbar promotion, title
fallbacks and anything else that exists to match `SwiftUI` or the old Swift
backend belongs there, never here. When a host needs a specific choice
(e.g. a badge's capsule metrics or a progress spinner's control size),
expose it as a plain parameter or generic wrapper and let the backend
supply the value — don't bake host chrome into the kit.

## API rules

- Main-thread types are created and used only with a `MainThreadMarker`.
- Native objects are owned through `Retained`.
- No `unsafe` function or `msg_send!` appears in the public API; `unsafe`
  and `msg_send!` stay inside the kit, and each module that uses them
  documents why in a `# Safety` section.
- A panic in any closure the frameworks call is logged and aborts the
  process, instead of unwinding into Objective-C.
- The crate knows nothing about any particular UI toolkit: no WaterUI, nami
  or other host types, identifiers or call-site-specific introspection.
