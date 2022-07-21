use wayland_client::event_created_child;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_v1::ZwlrOutputConfigurationV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_head_v1::ZwlrOutputHeadV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_manager_v1::ZwlrOutputManagerV1;
use wayland_protocols_wlr::output_management::v1::client::zwlr_output_mode_v1::ZwlrOutputModeV1;

#[derive(Default, Debug)]
struct ShikaneState {
    done: bool,
    wlr_output_manager: Option<ZwlrOutputManagerV1>,
    wlr_output_heads: Vec<ZwlrOutputHeadV1>,
}

type Data = ();

impl Dispatch<wl_registry::WlRegistry, Data> for ShikaneState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qhandle: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match &interface[..] {
                "zwlr_output_manager_v1" => {
                    let wlr_output_manager = registry
                        .bind::<ZwlrOutputManagerV1, _, _>(name, version, qhandle, ())
                        .unwrap();
                    state.wlr_output_manager = Some(wlr_output_manager);
                    println!("[{}] {} (v{})", name, interface, version);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<ZwlrOutputManagerV1, Data> for ShikaneState {
    fn event(
        state: &mut Self,
        _proxy: &ZwlrOutputManagerV1,
        event: <ZwlrOutputManagerV1 as wayland_client::Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head } => {
                state.wlr_output_heads.push(head);
                dbg!("Head Creation");
            }
            zwlr_output_manager_v1::Event::Done { serial } => {
                dbg!(serial);
                dbg!("Done");
                state.done = true;
            }
            zwlr_output_manager_v1::Event::Finished => {
                println!("ZwlrOutputManager::Event::Finished")
            }
            _ => panic!(),
        }
    }

    event_created_child!(ShikaneState, ZwlrOutputManagerV1, [
        0 => (ZwlrOutputHeadV1, ()),
    ]);
}

impl Dispatch<ZwlrOutputHeadV1, Data> for ShikaneState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputHeadV1,
        event: <ZwlrOutputHeadV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        dbg!(event);
    }

    event_created_child!(ShikaneState, ZwlrOutputManagerV1, [
        3 => (ZwlrOutputModeV1, ()),
    ]);
}

impl Dispatch<ZwlrOutputModeV1, ()> for ShikaneState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputModeV1,
        event: <ZwlrOutputModeV1 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        dbg!(event);
    }
}

impl Dispatch<ZwlrOutputConfigurationHeadV1, Data> for ShikaneState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationHeadV1,
        event: <ZwlrOutputConfigurationHeadV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        dbg!(event);
    }
}

impl Dispatch<ZwlrOutputConfigurationV1, Data> for ShikaneState {
    fn event(
        _state: &mut Self,
        _proxy: &ZwlrOutputConfigurationV1,
        event: <ZwlrOutputConfigurationV1 as Proxy>::Event,
        _data: &Data,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        dbg!(event);
    }
}

fn main() {
    let conn = Connection::connect_to_env().unwrap();

    let display = conn.display();

    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();

    let _registry = display.get_registry(&qh, ()).unwrap();

    let mut state = ShikaneState::default();

    event_queue.roundtrip(&mut state).unwrap();

    while !state.done {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
    dbg!(state);
}
