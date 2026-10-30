import { useState } from "react";
import { SafeAreaView, ScrollView, StatusBar, Text, Pressable, View } from "react-native";
import { ApprovalScreen } from "./src/screens/ApprovalScreen";
import { ControlsScreen } from "./src/screens/ControlsScreen";
import { DevicesScreen } from "./src/screens/DevicesScreen";
import { ManualIpScreen, QrScreen } from "./src/screens/OnboardingScreens";
import { SettingsScreen } from "./src/screens/SettingsScreen";
import { navigationTitle } from "./src/navigation/routes";
import type { Route } from "./src/navigation/routes";
import { palette, styles } from "./src/ui/theme";

const tabs: ReadonlyArray<Readonly<{ route: Route; label: string }>> = [
  { route: "devices", label: "Dispositivos" },
  { route: "controls", label: "Controles" },
  { route: "settings", label: "Ajustes" },
];

export default function App() {
  const [route, setRoute] = useState<Route>("devices");
  const goDevices = () => setRoute("devices");

  return (
    <SafeAreaView style={styles.root}>
      <StatusBar barStyle="light-content" />
      <View style={styles.topBar}>
        <Text style={styles.wordmark}>zoo<Text style={{ color: palette.primary }}>mouse</Text></Text>
        <Text style={styles.status}>LAN · SIN CONEXIÓN</Text>
      </View>
      <ScrollView
        key={route}
        keyboardShouldPersistTaps="handled"
        contentContainerStyle={styles.scroll}
        style={styles.page}
        accessibilityLabel={navigationTitle(route)}
      >
        {route === "devices" ? <DevicesScreen onIp={() => setRoute("manual-ip")} onQr={() => setRoute("qr")} onApproval={() => setRoute("approval")} /> : null}
        {route === "manual-ip" ? <ManualIpScreen onBack={goDevices} /> : null}
        {route === "qr" ? <QrScreen onBack={goDevices} /> : null}
        {route === "approval" ? <ApprovalScreen onBack={goDevices} /> : null}
        {route === "controls" ? <ControlsScreen onApproval={() => setRoute("approval")} onSettings={() => setRoute("settings")} /> : null}
        {route === "settings" ? <SettingsScreen /> : null}
      </ScrollView>
      <View style={styles.nav} accessibilityRole="tablist">
        {tabs.map(tab => (
          <Pressable key={tab.route} accessibilityRole="tab" accessibilityState={{ selected: route === tab.route }}
            onPress={() => setRoute(tab.route)} style={styles.navItem}>
            <Text style={[styles.navText, route === tab.route && styles.navTextActive]}>{tab.label}</Text>
          </Pressable>
        ))}
      </View>
    </SafeAreaView>
  );
}
