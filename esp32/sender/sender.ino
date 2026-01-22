#include <SPI.h>
#include <LoRa.h>
#include <WiFi.h>
#include <WiFiUdp.h>
#include <NTPClient.h>

//define the pins used by the transceiver module
#define ss 5
#define rst 14
#define dio0 2

const char* ssid = "";
const char* password = "";

// NTP Client setup
WiFiUDP ntpUDP;
// UTC+8 for Malaysia/Singapore, adjust offset as needed (in seconds)
// UTC+8 = 8 * 3600 = 28800
NTPClient timeClient(ntpUDP, "pool.ntp.org", 28800, 60000);

void connectWiFi() {
  Serial.println();
  Serial.println("Connecting to WiFi...");
  WiFi.begin(ssid, password);

  int retryCount = 0;
  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
    retryCount++;

    if (retryCount >= 20) { // ~10 seconds timeout
      Serial.println("\n⚠️ WiFi connection failed. Retrying...");
      WiFi.disconnect();
      delay(2000);
      WiFi.begin(ssid, password);
      retryCount = 0;
    }
  }
  Serial.println("\nWiFi connected");
  Serial.println("IP address: ");
  Serial.println(WiFi.localIP());
}

int counter = 0;

void setup() {
  //initialize Serial Monitor
  Serial.begin(115200);
  while (!Serial);
  
  connectWiFi();
  
  // Initialize NTP client
  timeClient.begin();
  Serial.println("Syncing time with NTP server...");
  while (!timeClient.update()) {
    timeClient.forceUpdate();
    delay(500);
  }
  Serial.println("Time synchronized!");
  Serial.print("Current time: ");
  Serial.println(timeClient.getFormattedTime());
  
  Serial.println("LoRa Sender");

  //setup LoRa transceiver module
  LoRa.setPins(ss, rst, dio0);
  
  //replace the LoRa.begin(---E-) argument with your location's frequency 
  //433E6 for Asia
  //868E6 for Europe
  //915E6 for North America
  while (!LoRa.begin(433E6)) {
    Serial.println(".");
    delay(500);
  }
  // Change sync word (0xF3) to match the receiver
  // The sync word assures you don't get LoRa messages from other LoRa transceivers
  // ranges from 0-0xFF
  LoRa.setSyncWord(0xF3);
  Serial.println("LoRa Initializing OK!");
}

void loop() {
  // Update time from NTP
  timeClient.update();
  
  String currentTime = timeClient.getFormattedTime();
  
  Serial.print("Sending packet #");
  Serial.print(counter);
  Serial.print(" at ");
  Serial.println(currentTime);

  // Send LoRa packet to receiver with timestamp
  // Format: "TIME:HH:MM:SS|DATA:hello X"
  LoRa.beginPacket();
  LoRa.print("TIME:");
  LoRa.print(currentTime);
  LoRa.print("|DATA:hello ");
  LoRa.print(counter);
  LoRa.endPacket();

  counter++;

  delay(10000);
}