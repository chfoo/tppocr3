#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

pub mod vnc {
    include!(concat!(env!("OUT_DIR"), "/libvnc_bindings.rs"));
}
