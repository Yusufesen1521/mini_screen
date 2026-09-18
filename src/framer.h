// framer.h - gelen bayt akisindan cerceve ayristirici
//
// Senkron arama kaydirmali kayitla yapiliyor: her bayt icin son 10 bayt
// penceresine bakilir, SOF ve baslik CRC'si birlikte tutarsa cerceve
// baslamis sayilir. Bu yaklasim ust uste binen sahte SOF durumunda gercek
// SOF'u kacirmaz, ayrica ayri bir yeniden hizalanma koduna gerek birakmaz.
//
// Baslik CRC'si sadece SOF eslestiginde hesaplanir, yani senkron ararken
// bile maliyeti ihmal edilebilir.

#pragma once

#include <stddef.h>
#include <stdint.h>

#include "protocol.h"

class Framer {
public:
  typedef void (*FrameFn)(uint8_t type, uint8_t flags, uint8_t seq,
                          const uint8_t *payload, uint16_t len, void *ctx);
  typedef void (*ErrorFn)(uint8_t reason, uint8_t seq, void *ctx);

  struct Stats {
    uint32_t framesOk;
    uint32_t framesDropped;
    uint16_t hdrCrcErrors;
    uint16_t payloadCrcErrors;
    uint16_t syncLosses;
  };

  void begin(uint8_t *payloadBuf, uint16_t payloadCap,
             FrameFn onFrame, ErrorFn onError, void *ctx);

  void feed(const uint8_t *data, size_t len, uint32_t nowMs);

  // Yarim kalmis cerceveyi zaman asimina ugratmak icin duzenli cagrilir.
  void poll(uint32_t nowMs);

  const Stats &stats() const { return _stats; }

private:
  enum State : uint8_t { HUNT, PAYLOAD, TRAILER };

  void feedByte(uint8_t b);
  void reset();

  uint8_t  *_buf = nullptr;
  uint16_t  _cap = 0;
  FrameFn   _onFrame = nullptr;
  ErrorFn   _onError = nullptr;
  void     *_ctx = nullptr;

  State    _state = HUNT;
  uint8_t  _win[PROTO_HDR_SIZE] = {0};
  uint8_t  _winLen = 0;

  uint8_t  _type = 0;
  uint8_t  _flags = 0;
  uint8_t  _seq = 0;
  uint16_t _len = 0;
  uint16_t _pos = 0;
  uint8_t  _trailer[PROTO_CRC_SIZE] = {0};
  uint8_t  _trailerPos = 0;

  uint32_t _lastByteMs = 0;
  Stats    _stats = {};
};
