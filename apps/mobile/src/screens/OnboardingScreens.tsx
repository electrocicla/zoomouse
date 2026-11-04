import { useEffect, useRef, useState } from "react";
import { Text, TextInput, View } from "react-native";
import { parsePrivateIpv4 } from "../domain/network";
import { LanAttemptCoordinator } from "../domain/lan-attempt";
import type { LanAttemptState } from "../domain/lan-attempt";
import { Action, Panel } from "../ui/Elements";
import { palette, styles } from "../ui/theme";

export const ManualIpScreen = ({ coordinator, onBack }: {
  coordinator: LanAttemptCoordinator;
  onBack: () => void;
}) => {
  const [ip, setIp] = useState("");
  const [portText, setPortText] = useState("");
  const [state, setState] = useState<LanAttemptState>(coordinator.state);
  const mounted = useRef(true);
  const normalized = parsePrivateIpv4(ip);
  const port = /^[1-9][0-9]{0,4}$/.test(portText) ? Number(portText) : 0;
  const validPort = port > 0 && port <= 65535;
  const busy = state.phase === "opening" || state.phase === "closing";

  useEffect(() => {
    mounted.current = true;
    setState(coordinator.state);
    return () => {
      mounted.current = false;
      // The App owns the coordinator across navigation and can observe
      // late native completions and cleanup errors after this screen exits.
      void coordinator.disconnect();
    };
  }, [coordinator]);

  const inspect = async (): Promise<void> => {
    if (normalized === null || !validPort || busy) return;
    const pending = coordinator.open({ kind: "manual-ip", ipv4: normalized, port });
    setState(coordinator.state);
    const result = await pending;
    if (mounted.current) setState(result);
  };

  const resultLabel = state.phase === "opening"
    ? "Se está verificando la disponibilidad del adaptador nativo."
    : state.phase === "closing"
      ? "Cerrando el intento anterior; no se permite reconectar todavía."
      : state.phase === "blocked"
        ? state.reason === "cleanup-failed"
          ? "La limpieza nativa falló. Reinicia la aplicación antes de intentar otra conexión."
          : state.reason === "invalid-candidate"
            ? "La dirección o el puerto no son válidos."
            : "No hay un canal Android DTLS autenticado disponible. No se envió ningún control."
        : "Introducir IP y puerto no autentica al host ni habilita el control remoto.";

  return (
    <View>
      <Text style={styles.overline}>CONEXIÓN LAN · SIN AUTORIZACIÓN</Text>
      <Text style={styles.title}>Verificar IP del equipo</Text>
      <Text style={styles.subtitle}>Solo se aceptan destinos privados. El módulo nativo está deshabilitado hasta contar con prueba criptográfica y consentimiento real de Windows.</Text>
      <Panel title="Destino IPv4 del equipo" badge="CANAL NO DISPONIBLE">
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
        <TextInput
          style={styles.input}
          placeholder="Puerto del host (p. ej. 50001)"
          placeholderTextColor={palette.disabled}
          value={portText}
          onChangeText={setPortText}
          keyboardType="numeric"
          maxLength={5}
          accessibilityLabel="Puerto LAN del computador"
        />
        <Text style={[styles.body, { marginTop: 12 }]}>
          {ip.length === 0 ? "Introduce una dirección RFC1918." : normalized === null ? "IPv4 privada inválida." : "Dirección válida, todavía no autenticada."}
        </Text>
        <Text style={styles.body}>{resultLabel}</Text>
        <Action
          label={busy ? "Verificación en curso" : "Verificar canal seguro (sin conexión)"}
          onPress={() => { void inspect(); }}
          disabled={normalized === null || !validPort || busy || state.phase === "native-link-unverified" ||
            (state.phase === "blocked" && state.reason === "cleanup-failed")}
        />
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
