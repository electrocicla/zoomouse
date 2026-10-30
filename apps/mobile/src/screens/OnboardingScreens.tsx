import { useState } from "react";
import { Text, TextInput, View } from "react-native";
import { parsePrivateIpv4 } from "../domain/network";
import { Action, Panel } from "../ui/Elements";
import { palette, styles } from "../ui/theme";

export const ManualIpScreen = ({ onBack }: { onBack: () => void }) => {
  const [ip, setIp] = useState("");
  const normalized = parsePrivateIpv4(ip);
  return (
    <View>
      <Text style={styles.overline}>CONEXIÓN LAN</Text>
      <Text style={styles.title}>Conectar por IP</Text>
      <Text style={styles.subtitle}>Validación local de direcciones privadas. No hay conexión de red ni solicitud de permisos en esta fase.</Text>
      <Panel title="Dirección IPv4 del equipo" badge="SIN TRANSPORTE">
        <TextInput
          style={styles.input}
          placeholder="192.168.1.100"
          placeholderTextColor={palette.disabled}
          value={ip}
          onChangeText={setIp}
          keyboardType="numeric"
          autoCapitalize="none"
          autoCorrect={false}
          maxLength={15}
          accessibilityLabel="IPv4 privada del computador"
        />
        <Text style={[styles.body, { marginTop: 12 }]}>
          {ip.length === 0 ? "Utiliza una dirección RFC1918 de tu red local." : normalized ? "Formato válido. La conexión se habilitará en M1-D." : "Introduce una dirección LAN privada válida."}
        </Text>
        <Action label="Solicitar conexión (aún no disponible)" onPress={() => {}} disabled />
      </Panel>
      <Action label="Volver a dispositivos" onPress={onBack} secondary />
    </View>
  );
};

export const QrScreen = ({ onBack }: { onBack: () => void }) => (
  <View>
    <Text style={styles.overline}>EMPAREJAMIENTO</Text>
    <Text style={styles.title}>Escanear QR</Text>
    <Text style={styles.subtitle}>El futuro código firmado será generado por el host Windows para identificarlo y solicitar consentimiento.</Text>
    <Panel title="Escáner no instalado" badge="SIN CÁMARA">
      <Text style={styles.body}>Zoomouse no pide permisos de cámara en M1-B. El escáner y la verificación criptográfica llegan en M1-D.</Text>
      <Action label="Abrir cámara (próximamente)" onPress={() => {}} disabled />
    </Panel>
    <Action label="Volver a dispositivos" onPress={onBack} secondary />
  </View>
);
