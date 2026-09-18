// backlight.h - arka isik, LEDC PWM ile
//
// TFT_eSPI'ye birakilmadi: TFT_BL tanimlanmiyor, pin dogrudan burada surulyor.

#pragma once

#include <stdint.h>

void backlightBegin();

// brightness: 0 = kapali, 255 = tam parlaklik (8 bit cozunurluk ile birebir)
void backlightSet(uint8_t brightness);

// Ekran baslatilmadan once kisa darbeler. Panel hic goruntu vermese bile bu
// darbeler goruluyorsa firmware calisiyor, GPIO 21 / VCC / GND saglam demektir.
void backlightHeartbeat();
