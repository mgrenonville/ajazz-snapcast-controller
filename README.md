```
  ┌─────────────────────────────────────────────────────────────────────┐
  │                         MAIN EVENT LOOP                              │
  │                         (main.rs:78-135)                            │
  │                                                                      │
  │  ┌────────────────────────────────────────────────────────────┐    │
  │  │            ApplicationState (in-memory)                     │    │
  │  │  - room: RoomState (volume, muted, stream_id, etc.)        │    │
  │  │  - streams: Vec<AudioStream>                               │    │
  │  │  - hardware_connected: bool                                │    │
  │  │  - server_connected: bool                                  │    │
  │  └────────────────────────────────────────────────────────────┘    │
  │                                                                      │
  └─────────────────────────────────────────────────────────────────────┘
             ▲        │                    ▲        │
             │        │                    │        │
             │        │                    │        │
       ┌─────┘        └─────┐        ┌────┘        └────┐
       │                    │        │                   │
       │ HardwareEvent      │        │ SnapcastEvent     │
       │ (unbounded_channel)│        │ (unbounded_channel)│
       │                    │        │                   │
       │              HardwareCommand│          SnapcastCommand
       │              (unbounded_     │          (unbounded_
       │               channel)       │           channel)
       │                    │        │                   │
       │                    ▼        │                   ▼
       │                             │
  ┌────┴─────────────────┐          │          ┌──────────────────────┐
  │  HARDWARE TASK       │          │          │  SNAPCAST TASK       │
  │  (tokio::spawn)      │          │          │  (async, pinned)     │
  │                      │          │          │                      │
  │  DeviceConnection    │          │          │  ConnectionHandler   │
  │  Handler             │          │          │                      │
  │                      │          │          │                      │
  │  ┌────────────────┐ │          │          │  ┌────────────────┐  │
  │  │ Event Polling  │ │          │          │  │ Message Loop   │  │
  │  │ (10ms interval)│ │          │          │  │                │  │
  │  └────────────────┘ │          │          │  └────────────────┘  │
  │         │            │          │          │         │            │
  │         │ read       │          │          │         │ recv       │
  │         │ events     │          │          │         │ notif.     │
  │         ▼            │          │          │         ▼            │
  │  ┌────────────────┐ │          │          │  ┌────────────────┐  │
  │  │ AsyncDevice    │ │          │          │  │ SnapcastClient │  │
  │  │ StateReader    │ │          │          │  │                │  │
  │  └────────────────┘ │          │          │  └────────────────┘  │
  │         │            │          │          │         │            │
  │         │            │          │          │         │            │
  │         ▼            │          │          │         ▼            │
  │  ┌────────────────┐ │          │          │  ┌────────────────┐  │
  │  │ USB HID Device │ │          │          │  │ TCP Connection │  │
  │  │ (Ajazz)        │ │          │          │  │ to Snapcast    │  │
  │  └────────────────┘ │          │          │  │ Server         │  │
  │         ▲            │          │          │  └────────────────┘  │
  │         │            │          │          │         ▲            │
  │         │ render     │          │          │         │ send       │
  │         │ screens    │          │          │         │ RPC        │
  │         │            │          │          │         │            │
  │  ┌────────────────┐ │          │          │  ┌────────────────┐  │
  │  │ DisplayManager │◄├──────────┘          └──┤ handle_command │  │
  │  │ (render status)│ │                         │ (execute RPC)  │  │
  │  └────────────────┘ │                         └────────────────┘  │
  │                      │                                             │
  └──────────────────────┘                         └──────────────────┘
```