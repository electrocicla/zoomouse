import { Text, View } from "react-native";
import { Action, Panel } from "../ui/Elements";
import { styles } from "../ui/theme";

export const DevicesScreen = ({ onIp, onQr, onApproval }: {
  onIp: () => void;
  onQr: () => void;
  onApproval: () => void;
}) => (
  <View>
    <Text style={styles.overline}>RED LOCAL · FASE M1</Text>
    <Text style={styles.title}>Tu PC, desde aquí.</Text>
    <Text style={styles.subtitle}>Conexión LAN privada con autorización obligatoria en Windows. La búsqueda automática todavía no está implementada.</Text>
    <Panel title="Equipos detectados" badge="NO DISPONIBLE">
      <Text style={styles.body}>No se ha iniciado mDNS ni se han encontrado dispositivos. Esta pantalla no usa datos ficticios.</Text>
    </Panel>
    <Panel title="Agregar un computador">
      <Text style={styles.body}>Ingresa la IP privada del host o consulta el futuro flujo de emparejamiento QR.</Text>
      <Action label="Introducir dirección IP" onPress={onIp} />
      <Action label="Emparejar con QR (próximamente)" onPress={onQr} secondary />
    </Panel>
    <Action label="Cómo funcionará la aprobación" onPress={onApproval} secondary />
  </View>
);
