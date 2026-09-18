// log.h - log ciktisi, sadece UART0 uzerine
//
// USB CDC portu (Serial) artik protokole ait. Oraya bir bayt log yazmak
// cerceve akisini bozar, bu yuzden log yalnizca Serial0 uzerine gider.
//
// Pratik sonuc: kartin iki USB portu da takili olmali.
//   UART kopru portu -> yukleme ve log
//   yerlesik USB portu -> protokol

#pragma once

void logBegin();
void logPrintf(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
