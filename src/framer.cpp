#include "framer.h"

#include <string.h>

#include "crc.h"

void Framer::begin(uint8_t *payloadBuf, uint16_t payloadCap,
                   FrameFn onFrame, ErrorFn onError, void *ctx)
{
  _buf = payloadBuf;
  _cap = payloadCap;
  _onFrame = onFrame;
  _onError = onError;
  _ctx = ctx;
  reset();
}

void Framer::reset()
{
  _state = HUNT;
  _winLen = 0;
  _pos = 0;
  _trailerPos = 0;
}

void Framer::feed(const uint8_t *data, size_t len, uint32_t nowMs)
{
  size_t i = 0;

  while (i < len) {
    // Payload durumunda bayt bayt ilerlemek gereksiz, toplu kopyala.
    if (_state == PAYLOAD) {
      const size_t need = (size_t)_len - _pos;
      const size_t avail = len - i;
      const size_t n = (need < avail) ? need : avail;

      memcpy(&_buf[_pos], &data[i], n);
      _pos += (uint16_t)n;
      i += n;

      if (_pos == _len) {
        _state = TRAILER;
      }
      continue;
    }

    feedByte(data[i++]);
  }

  if (len > 0) {
    _lastByteMs = nowMs;
  }
}

void Framer::feedByte(uint8_t b)
{
  if (_state == TRAILER) {
    _trailer[_trailerPos++] = b;
    if (_trailerPos < PROTO_CRC_SIZE) {
      return;
    }

    const uint16_t received = (uint16_t)(_trailer[0] | (_trailer[1] << 8));
    const uint16_t computed = crc16(_buf, _len);

    if (received == computed) {
      _stats.framesOk++;
      if (_onFrame) {
        _onFrame(_type, _flags, _seq, _buf, _len, _ctx);
      }
    } else {
      _stats.payloadCrcErrors++;
      _stats.framesDropped++;
      if (_onError) {
        _onError(ERR_BAD_PAYLOAD_CRC, _seq, _ctx);
      }
    }

    reset();
    return;
  }

  // HUNT: son 10 bayti tutan kaydirmali pencere
  if (_winLen < PROTO_HDR_SIZE) {
    _win[_winLen++] = b;
  } else {
    memmove(_win, &_win[1], PROTO_HDR_SIZE - 1);
    _win[PROTO_HDR_SIZE - 1] = b;
  }

  if (_winLen < PROTO_HDR_SIZE) {
    return;
  }
  if (_win[PROTO_OFF_SOF0] != PROTO_SOF0 || _win[PROTO_OFF_SOF1] != PROTO_SOF1) {
    return;
  }
  if (crc8(&_win[PROTO_HDRCRC_START], PROTO_HDRCRC_LEN) != _win[PROTO_OFF_HDRCRC]) {
    // SOF eslesti ama baslik bozuk. Pencere bir bayt kayarak devam edecegi
    // icin ust uste binen gercek bir SOF kacmaz.
    _stats.hdrCrcErrors++;
    return;
  }

  _type = _win[PROTO_OFF_TYPE];
  _flags = _win[PROTO_OFF_FLAGS];
  _seq = _win[PROTO_OFF_SEQ];
  _len = (uint16_t)(_win[PROTO_OFF_LEN] | (_win[PROTO_OFF_LEN + 1] << 8));

  const uint8_t version = _win[PROTO_OFF_VER];
  _winLen = 0;

  if (version != PROTO_VERSION) {
    _stats.framesDropped++;
    if (_onError) {
      _onError(ERR_BAD_VERSION, _seq, _ctx);
    }
    return;
  }

  if (_len > _cap) {
    _stats.framesDropped++;
    if (_onError) {
      _onError(ERR_PAYLOAD_TOO_LARGE, _seq, _ctx);
    }
    return;
  }

  _pos = 0;
  _trailerPos = 0;
  _state = (_len == 0) ? TRAILER : PAYLOAD;
}

void Framer::poll(uint32_t nowMs)
{
  if (_state == HUNT) {
    return;
  }
  if ((nowMs - _lastByteMs) < PROTO_FRAME_TIMEOUT_MS) {
    return;
  }

  _stats.syncLosses++;
  _stats.framesDropped++;
  reset();
}
