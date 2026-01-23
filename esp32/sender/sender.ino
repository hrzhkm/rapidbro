#include <SPI.h>
#include <LoRa.h>
#include <Wire.h>
#include <WiFi.h>
#include <WiFiUdp.h>
#include <NTPClient.h>
#include <Adafruit_GFX.h>
#include <Adafruit_SH110X.h>

//define the pins used by the transceiver module
#define ss 5
#define rst 14
#define dio0 2

// OLED display settings (using default I2C pins)
#define OLED_SDA 21
#define OLED_SCL 22
#define SCREEN_WIDTH 128
#define SCREEN_HEIGHT 64
#define OLED_RESET -1

Adafruit_SH1106G display = Adafruit_SH1106G(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, OLED_RESET);

const char* ssid = "HOTSPORT25";
const char* password = "ZC224141";

// NTP Client setup
WiFiUDP ntpUDP;
// UTC+8 for Malaysia/Singapore, adjust offset as needed (in seconds)
// UTC+8 = 8 * 3600 = 28800
NTPClient timeClient(ntpUDP, "pool.ntp.org", 28800, 60000);

void updateDisplay(String status, String line1, String line2, String line3, String line4) {
  display.clearDisplay();
  display.setCursor(0, 0);
  display.println("LoRa Sender");
  display.print("STATUS: ");
  display.println(status);
  display.println("----------------");
  if (line1.length() > 0) display.println(line1);
  if (line2.length() > 0) display.println(line2);
  if (line3.length() > 0) display.println(line3);
  if (line4.length() > 0) display.println(line4);
  display.display();
}

void connectWiFi() {
  Serial.println();
  Serial.println("Connecting to WiFi...");
  WiFi.begin(ssid, password);

  int retryCount = 0;
  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
    retryCount++;
    
    // Update OLED with WiFi connection status
    String dots = "";
    for (int i = 0; i < (retryCount % 4); i++) dots += ".";
    updateDisplay("WIFI" + dots, "Connecting to WiFi", "SSID: " + String(ssid), "Attempt: " + String(retryCount), "");

    if (retryCount >= 20) { // ~10 seconds timeout
      Serial.println("\nWiFi connection failed. Retrying...");
      updateDisplay("RETRY", "WiFi failed!", "Retrying...", "", "");
      WiFi.disconnect();
      delay(2000);
      WiFi.begin(ssid, password);
      retryCount = 0;
    }
  }
  Serial.println("\nWiFi connected");
  Serial.println("IP address: ");
  Serial.println(WiFi.localIP());
  
  updateDisplay("WIFI OK", "WiFi Connected!", "IP: " + WiFi.localIP().toString(), "", "");
  delay(1000);
}

int counter = 0;

void setup() {
  //initialize Serial Monitor
  Serial.begin(115200);
  while (!Serial);
  
  // Initialize I2C and OLED display
  Wire.begin(OLED_SDA, OLED_SCL);
  display.begin(0x3C, true);
  display.clearDisplay();
  display.setTextSize(1);
  display.setTextColor(SH110X_WHITE);
  
  // Show initializing message
  updateDisplay("INIT", "Initializing...", "", "", "");
  delay(500);
  
  connectWiFi();
  
  // Initialize NTP client
  timeClient.begin();
  Serial.println("Syncing time with NTP server...");
  updateDisplay("NTP SYNC", "Syncing time...", "Server: pool.ntp.org", "", "");
  
  int ntpAttempts = 0;
  while (!timeClient.update()) {
    timeClient.forceUpdate();
    ntpAttempts++;
    String dots = "";
    for (int i = 0; i < (ntpAttempts % 4); i++) dots += ".";
    updateDisplay("NTP" + dots, "Syncing time...", "Attempt: " + String(ntpAttempts), "", "");
    delay(500);
  }
  Serial.println("Time synchronized!");
  Serial.print("Current time: ");
  Serial.println(timeClient.getFormattedTime());
  
  updateDisplay("NTP OK", "Time synced!", "Time: " + timeClient.getFormattedTime(), "", "");
  delay(1000);
  
  Serial.println("LoRa Sender");

  //setup LoRa transceiver module
  LoRa.setPins(ss, rst, dio0);
  
  //replace the LoRa.begin(---E-) argument with your location's frequency 
  //433E6 for Asia
  //868E6 for Europe
  //915E6 for North America
  updateDisplay("LORA INIT", "Starting LoRa...", "Freq: 433 MHz", "", "");
  
  int loraAttempts = 0;
  while (!LoRa.begin(433E6)) {
    Serial.println(".");
    loraAttempts++;
    String dots = "";
    for (int i = 0; i < (loraAttempts % 4); i++) dots += ".";
    updateDisplay("LORA" + dots, "Connecting LoRa...", "Attempt: " + String(loraAttempts), "", "");
    delay(500);
  }
  // Change sync word (0xF3) to match the receiver
  // The sync word assures you don't get LoRa messages from other LoRa transceivers
  // ranges from 0-0xFF
  LoRa.setSyncWord(0xF3);
  Serial.println("LoRa Initializing OK!");
  
  updateDisplay("READY", "All systems ready!", "WiFi: OK", "NTP: OK", "LoRa: OK");
  delay(2000);
}

void loop() {
  // Update time from NTP
  timeClient.update();
  
  String currentTime = timeClient.getFormattedTime();
  
  Serial.print("Sending packet #");
  Serial.print(counter);
  Serial.print(" at ");
  Serial.println(currentTime);

  // Update display - sending status
  updateDisplay("SENDING", "Time: " + currentTime, "Packet #" + String(counter), "Data: hello " + String(counter), "");

  // Send LoRa packet to receiver with timestamp
  // Format: "TIME:HH:MM:SS|DATA:hello X"
  LoRa.beginPacket();
  LoRa.print("TIME:");
  LoRa.print(currentTime);
  LoRa.print("|DATA:hello ");
  LoRa.print(counter);
  LoRa.endPacket();

  // Update display - sent confirmation
  updateDisplay("SENT", "Time: " + currentTime, "Packet #" + String(counter), "Data: hello " + String(counter), "Next in 10s...");

  counter++;

  delay(10000);
}