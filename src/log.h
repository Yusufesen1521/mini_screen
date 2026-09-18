// log.h - log ciktisi
//
// Iki hedefe birden yazar:
//   1. Serial0 (UART0). Kartin UART kopru portu takiliysa gorunur.
//   2. Protokol uzerinden MSG_LOG cerceveleri. main.cpp baglanti kurulunca
//      logSetSink ile bu hedefi kuruyor.
//
// USB CDC portuna (Serial) dogrudan yazilmaz, orasi protokole ait.
// Ikinci hedef sayesinde tek kabloyla, sadece USB portu takiliyken de
// tanilama gorulebiliyor.
//
// Hedef kurulmadan once uretilen satirlar LOG_BOOT_BUF_SIZE kadar bir
// tamponda birikir ve hedef kurulunca topluca gonderilir. Acilis ciktisi
// PC baglanmadan once uretildigi icin bu gerekli.

#pragma once

typedef void (*LogSink)(const char *text);

void logBegin();
void logPrintf(const char *fmt, ...) __attribute__((format(printf, 1, 2)));

// Hedefi kurar ve o ana kadar birikmis satirlari bir kerede gonderir.
void logSetSink(LogSink sink);
