// Buttplug Rust Source Code File - See https://buttplug.io for more info.
//
// Copyright 2016-2026 Nonpolynomial Labs LLC. All rights reserved.
//
// Licensed under the BSD 3-Clause license. See LICENSE file in the project root
// for full license information.

#[cfg(feature = "builtin-protocols")]
use crate::device::protocol::ProtocolIdentifier;

// Utility mods
pub mod fleshlight_launch_helper;

// Since users can pick and choose protocols, we need all of these to be public.
pub mod activejoy;
pub mod adrienlastic;
pub mod amorelie_joy;
pub mod aneros;
pub mod ankni;
pub mod bananasome;
pub mod cachito;
pub mod cowgirl;
pub mod cowgirl_cone;
pub mod cupido;
pub mod deepsire;
pub mod dglab;
pub mod feelingso;
pub mod fleshy_thrust;
pub mod fluffer;
pub mod fmachine;
pub mod foreo;
pub mod fox;
pub mod fredorch;
pub mod fredorch_rotary;
pub mod galaku;
pub mod galaku_pump;
pub mod hgod;
pub mod hismith;
pub mod hismith_mini;
pub mod honeyplaybox;
pub mod htk_bm;
pub mod itoys;
pub mod jejoue;
pub mod joyhub;
pub mod kgoal_boost;
pub mod kiiroo_powershot;
pub mod kiiroo_prowand;
pub mod kiiroo_spot;
pub mod kiiroo_spot_v2;
pub mod kiiroo_v2;
pub mod kiiroo_v21;
pub mod kiiroo_v21_initialized;
pub mod kiiroo_v2_vibrator;
pub mod kiiroo_v3;
pub mod kizuna;
pub mod lelo_harmony;
pub mod lelof1s;
pub mod lelof1sv2;
pub mod leten;
pub mod libo_elle;
pub mod libo_shark;
pub mod libo_vibes;
pub mod lioness;
pub mod loob;
pub mod lovedistance;
pub mod lovehoney_desire;
pub mod lovense;
pub mod lovense_connect_service;
pub mod lovenuts;
pub mod luvmazer;
pub mod magic_motion_v1;
pub mod magic_motion_v2;
pub mod magic_motion_v3;
pub mod magic_motion_v4;
pub mod mannuo;
pub mod maxpro;
pub mod meese;
pub mod mizzzee;
pub mod mizzzee_v2;
pub mod mizzzee_v3;
pub mod monsterpub;
pub mod motorbunny;
pub mod mymuselinkplus;
pub mod mysteryvibe;
pub mod mysteryvibe_v2;
pub mod nextlevelracing;
pub mod nexus_revo;
pub mod nobra;
pub mod omobo;
pub mod ossm;
pub mod patoo;
pub mod picobong;
pub mod pink_punch;
pub mod prettylove;
pub mod raw_protocol;
pub mod realov;
pub mod sakuraneko;
pub mod satisfyer;
pub mod sdl_gamepad;
pub mod sensee;
pub mod sensee_capsule;
pub mod sensee_v2;
pub mod serveu;
pub mod sexverse_lg389;
pub mod sexverse_v1;
pub mod sexverse_v2;
pub mod sexverse_v3;
pub mod sexverse_v4;
pub mod sexverse_v5;
pub mod sexverse_v6;
pub mod simulated;
pub mod svakom;
pub mod synchro;
pub mod tcode_v03;
pub mod thehandy;
pub mod thehandy_v3;
pub mod tingon;
pub mod tryfun;
pub mod tryfun_blackhole;
pub mod tryfun_meta2;
pub mod tryfun_rock;
pub mod umove;
pub mod utimi;
pub mod vibcrafter;
pub mod vibio;
pub mod vibratissimo;
pub mod vorze_sa;
pub mod wetoy;
pub mod wevibe;
pub mod wevibe8bit;
pub mod wevibe_chorus;
pub mod xibao;
pub mod xiuxiuda;
pub mod xuanhuan;
pub mod yiciyuan;
pub mod youcups;
pub mod youou;
pub mod zalo;

#[cfg(feature = "builtin-protocols")]
macro_rules! protocol_match {
  ( $var:ident, $( $( $protocol_module:ident )::+ ),* $(,)? ) => {
    match $var {
      $(
        $( $protocol_module )::+::setup::IDENTIFIER => Some($( $protocol_module )::+::setup::create_identifier()),
      )*
      _ => None,
    }
  };
}

// `protocol_factory!("<identifier>")`, one arm per protocol in the list below. `build.rs` reads
// the list from this file, so it stays the only list of protocols: keep one module per line.
include!(concat!(env!("OUT_DIR"), "/protocol_factory.rs"));

/// The protocol identifier for a protocol name from the device configuration, over every
/// protocol in this crate. The device manager uses it unless it is given another
/// [`ProtocolLookup`](crate::device::protocol::ProtocolLookup); one that names only some
/// protocols (built with [`protocol_factory!`](crate::protocol_factory)) lets the linker drop
/// the rest.
#[cfg(feature = "builtin-protocols")]
pub fn get_protocol_identifier(protocol_identifier: &str) -> Option<Box<dyn ProtocolIdentifier>> {
  protocol_match!(
    protocol_identifier,
    activejoy,
    adrienlastic,
    amorelie_joy,
    aneros,
    ankni,
    bananasome,
    cachito,
    cowgirl,
    cowgirl_cone,
    cupido,
    deepsire,
    dglab,
    hismith,
    hismith_mini,
    honeyplaybox,
    htk_bm,
    thehandy,
    thehandy_v3,
    feelingso,
    fleshy_thrust,
    fluffer,
    fmachine,
    foreo,
    fox,
    fredorch,
    fredorch_rotary,
    hgod,
    galaku_pump,
    galaku,
    itoys,
    jejoue,
    joyhub,
    kiiroo_powershot,
    kiiroo_prowand,
    kiiroo_spot,
    kiiroo_spot_v2,
    kiiroo_v2,
    kiiroo_v3,
    kiiroo_v2_vibrator,
    kiiroo_v21,
    kiiroo_v21_initialized,
    kizuna,
    lelof1s,
    lelof1sv2,
    leten,
    lelo_harmony,
    libo_elle,
    libo_shark,
    libo_vibes,
    lioness,
    loob,
    lovehoney_desire,
    lovedistance,
    lovense,
    lovense_connect_service,
    lovenuts,
    luvmazer,
    magic_motion_v1,
    magic_motion_v2,
    magic_motion_v3,
    magic_motion_v4,
    mannuo,
    maxpro,
    meese,
    mizzzee,
    mizzzee_v2,
    mizzzee_v3,
    svakom::mononii_skullman,
    monsterpub,
    motorbunny,
    mymuselinkplus,
    mysteryvibe,
    mysteryvibe_v2,
    nexus_revo,
    nextlevelracing,
    nobra,
    omobo,
    ossm,
    patoo,
    picobong,
    pink_punch,
    prettylove,
    raw_protocol,
    realov,
    sakuraneko,
    satisfyer,
    sensee,
    sensee_capsule,
    sensee_v2,
    sexverse_lg389,
    sexverse_v1,
    sexverse_v2,
    sexverse_v3,
    sexverse_v4,
    sexverse_v5,
    sexverse_v6,
    simulated,
    serveu,
    svakom::svakom_avaneo,
    svakom::svakom_alex,
    svakom::svakom_alex_v2,
    svakom::svakom_barnard,
    svakom::svakom_barney,
    svakom::svakom_dice,
    svakom::svakom_dt250a,
    svakom::svakom_fatima,
    svakom::svakom_iker,
    svakom::svakom_jordan,
    svakom::svakom_pulse,
    svakom::svakom_sam,
    svakom::svakom_sam2,
    svakom::svakom_suitcase,
    svakom::svakom_tarax,
    svakom::svakom_v1,
    svakom::svakom_v2,
    svakom::svakom_v3,
    svakom::svakom_v4,
    svakom::svakom_v5,
    svakom::svakom_v6,
    synchro,
    tingon,
    tryfun,
    tryfun_blackhole,
    tryfun_meta2,
    tryfun_rock,
    tcode_v03,
    umove,
    utimi,
    vibcrafter,
    vibio,
    vibratissimo,
    vorze_sa,
    wetoy,
    wevibe,
    wevibe8bit,
    wevibe_chorus,
    xibao,
    sdl_gamepad,
    xiuxiuda,
    xuanhuan,
    youcups,
    youou,
    yiciyuan,
    zalo,
    kgoal_boost,
  )
}
