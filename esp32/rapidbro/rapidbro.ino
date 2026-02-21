#include <WiFi.h>
#include <HTTPClient.h>
#include <Wire.h>
#include <Adafruit_GFX.h>
#include <Adafruit_SH110X.h>
#include <ArduinoJson.h>

// ===== Display (SH1106 OLED 128x64, I2C) =====
#define OLED_SDA 21
#define OLED_SCL 22
#define SCREEN_WIDTH 128
#define SCREEN_HEIGHT 64
#define OLED_RESET -1

Adafruit_SH1106G display(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, OLED_RESET);

// ===== WiFi =====
const char* WIFI_SSID = "YOUR_WIFI_SSID";
const char* WIFI_PASS = "YOUR_WIFI_PASSWORD";

// ===== Backend =====
// Use your backend host (must include protocol + port)
// Example: http://192.168.1.50:3030
const char* API_BASE_URL = "http://YOUR_BACKEND_IP:3030";
const char* ETA_ENDPOINT = "/get-t789-eta";

// ===== App timing =====
const unsigned long FETCH_INTERVAL_MS = 15000;  // refresh every 15s
const unsigned long WIFI_TIMEOUT_MS   = 15000;

// ===== Data model =====
struct EtaItem {
  String busNo;
  float etaMinutes;
  uint32_t stopsAway;
  float speedKmh;
};

EtaItem etaList[3];
int etaCount = 0;
String statusText = "Booting";
unsigned long lastFetchAt = 0;
unsigned long lastOkFetchAt = 0;

// --------- Tiny icon helpers ---------
void drawWifiIcon(int x, int y, bool connected) {
  if (!connected) {
    display.drawLine(x, y, x + 10, y + 10, SH110X_WHITE);
    display.drawLine(x + 10, y, x, y + 10, SH110X_WHITE);
    return;
  }

  display.drawCircle(x + 5, y + 8, 1, SH110X_WHITE);
  display.drawCircle(x + 5, y + 8, 4, SH110X_WHITE);
  display.drawCircle(x + 5, y + 8, 7, SH110X_WHITE);
}

void drawCloudApiIcon(int x, int y) {
  display.fillCircle(x + 6, y + 6, 4, SH110X_WHITE);
  display.fillCircle(x + 11, y + 5, 5, SH110X_WHITE);
  display.fillCircle(x + 16, y + 7, 4, SH110X_WHITE);
  display.fillRect(x + 6, y + 7, 11, 4, SH110X_WHITE);
}

void drawBusIcon(int x, int y) {
  display.drawRoundRect(x, y, 22, 12, 2, SH110X_WHITE);
  display.drawRect(x + 2, y + 2, 8, 4, SH110X_WHITE);
  display.drawRect(x + 12, y + 2, 8, 4, SH110X_WHITE);
  display.fillCircle(x + 5, y + 11, 2, SH110X_WHITE);
  display.fillCircle(x + 17, y + 11, 2, SH110X_WHITE);
}

void drawHeader() {
  display.fillRect(0, 0, SCREEN_WIDTH, 14, SH110X_WHITE);
  display.setTextColor(SH110X_BLACK);
  display.setTextSize(1);
  display.setCursor(4, 3);
  display.print("Rapidbro ETA  T789");

  // Icons at right side of header
  display.setTextColor(SH110X_BLACK);
  if (WiFi.status() == WL_CONNECTED) {
    // white bg header, draw black wifi with simple lines
    display.drawCircle(111, 8, 1, SH110X_BLACK);
    display.drawCircle(111, 8, 3, SH110X_BLACK);
    display.drawCircle(111, 8, 5, SH110X_BLACK);
  } else {
    display.drawLine(106, 3, 116, 13, SH110X_BLACK);
    display.drawLine(116, 3, 106, 13, SH110X_BLACK);
  }
  display.drawCircle(123, 8, 1, SH110X_BLACK); // tiny API dot indicator
}

void renderLoadingScreen(const String& msg) {
  display.clearDisplay();
  drawHeader();

  drawBusIcon(10, 24);
  drawCloudApiIcon(42, 24);
  drawWifiIcon(75, 24, WiFi.status() == WL_CONNECTED);

  display.setTextColor(SH110X_WHITE);
  display.setCursor(8, 50);
  display.print(msg);
  display.display();
}

void renderMainScreen() {
  display.clearDisplay();
  drawHeader();

  display.setTextColor(SH110X_WHITE);
  display.setTextSize(1);

  int y = 18;

  if (etaCount <= 0) {
    drawBusIcon(3, 18);
    display.setCursor(30, 20);
    display.print("No live buses yet");
    display.setCursor(30, 31);
    display.print("Status:");
    display.setCursor(30, 41);
    display.print(statusText);
  } else {
    for (int i = 0; i < etaCount; i++) {
      // ETA rank bubble
      display.drawRoundRect(2, y - 1, 14, 11, 3, SH110X_WHITE);
      display.setCursor(6, y + 1);
      display.print(i + 1);

      display.setCursor(20, y);
      display.print("Bus ");
      display.print(etaList[i].busNo);

      display.setCursor(76, y);
      display.print(etaList[i].etaMinutes, 1);
      display.print("m");

      display.setCursor(20, y + 9);
      display.print((int)etaList[i].stopsAway);
      display.print(" stop");
      if (etaList[i].stopsAway != 1) display.print("s");

      display.setCursor(76, y + 9);
      display.print(etaList[i].speedKmh, 0);
      display.print("km/h");

      y += 15;
      if (y > 50) break;
    }
  }

  // footer
  display.drawLine(0, 54, SCREEN_WIDTH, 54, SH110X_WHITE);
  display.setCursor(3, 57);
  display.print(statusText);

  if (lastOkFetchAt > 0) {
    unsigned long ageSec = (millis() - lastOkFetchAt) / 1000;
    display.setCursor(88, 57);
    display.print(ageSec);
    display.print("s");
  }

  display.display();
}

bool connectWiFi() {
  WiFi.mode(WIFI_STA);
  WiFi.begin(WIFI_SSID, WIFI_PASS);

  unsigned long start = millis();
  while (WiFi.status() != WL_CONNECTED && millis() - start < WIFI_TIMEOUT_MS) {
    renderLoadingScreen("Connecting WiFi...");
    delay(350);
  }

  if (WiFi.status() == WL_CONNECTED) {
    statusText = "WiFi connected";
    return true;
  }

  statusText = "WiFi failed";
  return false;
}

bool fetchT789Eta() {
  if (WiFi.status() != WL_CONNECTED) {
    statusText = "WiFi reconnecting";
    return false;
  }

  HTTPClient http;
  String url = String(API_BASE_URL) + ETA_ENDPOINT;

  http.setTimeout(8000);
  http.begin(url);
  int httpCode = http.GET();

  if (httpCode != HTTP_CODE_OK) {
    statusText = "API err " + String(httpCode);
    http.end();
    return false;
  }

  String payload = http.getString();
  http.end();

  DynamicJsonDocument doc(16 * 1024);
  DeserializationError err = deserializeJson(doc, payload);
  if (err) {
    statusText = "JSON parse fail";
    return false;
  }

  if (!doc.is<JsonArray>()) {
    statusText = "Bad API shape";
    return false;
  }

  JsonArray arr = doc.as<JsonArray>();
  etaCount = 0;

  for (JsonVariant v : arr) {
    if (etaCount >= 3) break;

    etaList[etaCount].busNo      = String((const char*)v["bus_no"] | "?");
    etaList[etaCount].etaMinutes = v["eta_minutes"] | 0.0;
    etaList[etaCount].stopsAway  = v["stops_away"] | 0;
    etaList[etaCount].speedKmh   = v["speed_kmh"] | 0.0;
    etaCount++;
  }

  statusText = "Updated";
  lastOkFetchAt = millis();
  return true;
}

void setup() {
  Serial.begin(115200);

  Wire.begin(OLED_SDA, OLED_SCL);
  display.begin(0x3C, true);
  display.clearDisplay();
  display.setTextColor(SH110X_WHITE);
  display.setTextSize(1);

  renderLoadingScreen("Starting Rapidbro...");
  delay(700);

  connectWiFi();
  renderLoadingScreen("Fetching ETA...");

  fetchT789Eta();
  renderMainScreen();
}

void loop() {
  if (WiFi.status() != WL_CONNECTED) {
    connectWiFi();
  }

  unsigned long now = millis();
  if (now - lastFetchAt >= FETCH_INTERVAL_MS) {
    lastFetchAt = now;
    fetchT789Eta();
    renderMainScreen();
  }

  // subtle live refresh (footer age, etc)
  static unsigned long lastUiTick = 0;
  if (now - lastUiTick >= 1000) {
    lastUiTick = now;
    renderMainScreen();
  }

  delay(20);
}
