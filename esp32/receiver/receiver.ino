#include <SPI.h>
#include <LoRa.h>
#include <Wire.h>
#include <Adafruit_GFX.h>
#include <Adafruit_SH110X.h>

//define the pins used by the transceiver module
#define ss 5
#define rst 14
#define dio0 2

// OLED display settings (using default I2C pins to avoid SPI conflict)
#define OLED_SDA 21  // Default ESP32 I2C SDA
#define OLED_SCL 22  // Default ESP32 I2C SCL
#define SCREEN_WIDTH 128
#define SCREEN_HEIGHT 64
#define OLED_RESET -1

Adafruit_SH1106G display = Adafruit_SH1106G(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, OLED_RESET);

void setup() {
  //initialize Serial Monitor
  Serial.begin(115200);
  while (!Serial);
  Serial.println("LoRa Receiver");

  // Initialize I2C with custom pins
  Wire.begin(OLED_SDA, OLED_SCL);
  
  // Initialize OLED display
  display.begin(0x3C, true);  // Address 0x3C, reset=true
  display.clearDisplay();
  display.setTextSize(1);
  display.setTextColor(SH110X_WHITE);
  
  // Show initializing message
  display.setCursor(0, 0);
  display.println("LoRa Receiver");
  display.println();
  display.println("Connecting...");
  display.display();

  //setup LoRa transceiver module
  LoRa.setPins(ss, rst, dio0);
  
  //replace the LoRa.begin(---E-) argument with your location's frequency 
  //433E6 for Asia
  //868E6 for Europe
  //915E6 for North America
  int attempts = 0;
  while (!LoRa.begin(433E6)) {
    Serial.println(".");
    attempts++;
    
    // Update OLED with connection attempts
    display.clearDisplay();
    display.setCursor(0, 0);
    display.println("LoRa Receiver");
    display.println();
    display.print("Connecting");
    for (int i = 0; i < (attempts % 4); i++) {
      display.print(".");
    }
    display.println();
    display.print("Attempt: ");
    display.println(attempts);
    display.display();
    
    delay(500);
  }
  
  // Change sync word (0xF3) to match the receiver
  // The sync word assures you don't get LoRa messages from other LoRa transceivers
  // ranges from 0-0xFF
  LoRa.setSyncWord(0xF3);
  Serial.println("LoRa Initializing OK!");
  
  // Show connected status on OLED
  display.clearDisplay();
  display.setCursor(0, 0);
  display.println("LoRa Receiver");
  display.println();
  display.println("STATUS: CONNECTED");
  display.println();
  display.println("Waiting for data...");
  display.display();
}

void loop() {
  // try to parse packet
  int packetSize = LoRa.parsePacket();
  if (packetSize) {
    // received a packet
    Serial.print("Received packet '");

    String LoRaData = "";
    // read packet
    while (LoRa.available()) {
      LoRaData = LoRa.readString();
      Serial.print(LoRaData); 
    }

    // print RSSI of packet
    int rssi = LoRa.packetRssi();
    Serial.print("' with RSSI ");
    Serial.println(rssi);
    
    // Display received data on OLED
    display.clearDisplay();
    display.setCursor(0, 0);
    display.println("LoRa Receiver");
    display.println("STATUS: CONNECTED");
    display.println("----------------");
    display.print("Data: ");
    display.println(LoRaData);
    display.println();
    display.print("RSSI: ");
    display.print(rssi);
    display.println(" dBm");
    display.display();
  }
}