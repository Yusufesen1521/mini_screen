#include "backlight.h"

#include <Arduino.h>

#include "pins.h"

void backlightBegin()
{
#if ESP_ARDUINO_VERSION_MAJOR >= 3
  ledcAttach(PIN_TFT_BL, BL_PWM_FREQ_HZ, BL_PWM_RESOLUTION_BITS);
#else
  ledcSetup(BL_PWM_CHANNEL, BL_PWM_FREQ_HZ, BL_PWM_RESOLUTION_BITS);
  ledcAttachPin(PIN_TFT_BL, BL_PWM_CHANNEL);
#endif
}

void backlightSet(uint8_t brightness)
{
#if ESP_ARDUINO_VERSION_MAJOR >= 3
  ledcWrite(PIN_TFT_BL, brightness);
#else
  ledcWrite(BL_PWM_CHANNEL, brightness);
#endif
}

void backlightHeartbeat()
{
  for (uint8_t i = 0; i < BL_HEARTBEAT_PULSES; i++) {
    backlightSet(BL_BRIGHTNESS_DEFAULT);
    delay(BL_HEARTBEAT_MS);
    backlightSet(BL_BRIGHTNESS_OFF);
    delay(BL_HEARTBEAT_MS);
  }
}
