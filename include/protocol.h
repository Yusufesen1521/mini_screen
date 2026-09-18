// protocol.h - USB CDC protokolunun paylasilan tanimlari
//
// Bayt duzeyinde tam tanim: docs/protocol.md
// Bu dosya o dokumanin makine tarafindaki karsiligi. Ikisi ayrilirsa
// dokuman dogru kabul edilir.

#pragma once

#include <stdint.h>

#define PROTO_VERSION           1

// ---------------------------------------------------------------------------
// Cerceve duzeni
// ---------------------------------------------------------------------------
#define PROTO_SOF0              0xA5
#define PROTO_SOF1              0x5A

#define PROTO_HDR_SIZE          10   // SOF'tan HDRCRC dahil
#define PROTO_CRC_SIZE          2

#define PROTO_OFF_SOF0          0
#define PROTO_OFF_SOF1          1
#define PROTO_OFF_VER           2
#define PROTO_OFF_TYPE          3
#define PROTO_OFF_FLAGS         4
#define PROTO_OFF_SEQ           5
#define PROTO_OFF_LEN           6    // 2 bayt, little-endian
#define PROTO_OFF_RSV           8
#define PROTO_OFF_HDRCRC        9

// HDRCRC, offset 2 ile 8 arasindaki 7 bayt uzerinde hesaplanir
#define PROTO_HDRCRC_START      PROTO_OFF_VER
#define PROTO_HDRCRC_LEN        7

#define PROTO_FLAG_ACK_REQ      0x01

// ---------------------------------------------------------------------------
// Mesaj tipleri
// ---------------------------------------------------------------------------
// PC -> cihaz
#define MSG_HELLO               0x01
#define MSG_FRAME_REGION        0x10
#define MSG_SET_BACKLIGHT       0x20
#define MSG_PING                0x30
#define MSG_GET_STATUS          0x40

// Cihaz -> PC
#define MSG_CAPS                0x81
#define MSG_ACK                 0x82
#define MSG_NACK                0x83
#define MSG_ERROR               0x84
#define MSG_STATUS              0x85
#define MSG_PONG                0x86

// ---------------------------------------------------------------------------
// Hata kodlari
// ---------------------------------------------------------------------------
#define ERR_BAD_HEADER_CRC      0x01
#define ERR_BAD_PAYLOAD_CRC     0x02
#define ERR_BAD_VERSION         0x03
#define ERR_UNKNOWN_TYPE        0x04
#define ERR_PAYLOAD_TOO_LARGE   0x05
#define ERR_BAD_REGION          0x06
#define ERR_BAD_CODEC           0x07
#define ERR_DECODE_ERROR        0x08
#define ERR_OVERRUN             0x09

// ---------------------------------------------------------------------------
// FRAME_REGION
// ---------------------------------------------------------------------------
#define REGION_HDR_SIZE         10

#define REGION_OFF_X            0
#define REGION_OFF_Y            2
#define REGION_OFF_W            4
#define REGION_OFF_H            6
#define REGION_OFF_FORMAT       8
#define REGION_OFF_CODEC        9

#define PIXFMT_RGB565_LE        0x01

#define CODEC_NONE              0x00
#define CODEC_RLE16             0x01

#define CODEC_MASK_NONE         0x01
#define CODEC_MASK_RLE16        0x02

// ---------------------------------------------------------------------------
// CAPS ve STATUS payload boyutlari
// ---------------------------------------------------------------------------
#define CAPS_PAYLOAD_SIZE       20
#define STATUS_PAYLOAD_SIZE     16
#define HELLO_PAYLOAD_SIZE      4

// ---------------------------------------------------------------------------
// Cihaz sinirlari
// ---------------------------------------------------------------------------
// LEN alani 16 bit. Cihaz bu kadarini tamponlayabildigini CAPS ile bildirir.
#define PROTO_MAX_PAYLOAD       65535

// Ayni anda tamponlanan cerceve sayisi. Tek gorevli alim yapiliyor, bu yuzden
// 1. Olcum kare kaybi gosterirse artirilacak.
#define PROTO_RX_SLOTS          1

// Cerceve yarim kalirsa bu sure sonunda birakilir
#define PROTO_FRAME_TIMEOUT_MS  500
