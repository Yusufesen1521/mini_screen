// log.h - iki seri port uzerine birden basan kucuk log yardimcisi
//
// ARDUINO_USB_CDC_ON_BOOT=1 oldugu icin Serial, kartin yerlesik USB portuna
// (GPIO 19/20) baglanir. Kart UART kopru portuna takiliysa oradan hicbir sey
// gorunmez. Bu yuzden ayni cikti Serial0 (UART0) uzerine de basiliyor;
// hangi porta takili olursan ol log akar.

#pragma once

void logBegin();
void logPrintf(const char *fmt, ...) __attribute__((format(printf, 1, 2)));
