import { Text, View } from "react-native";
import { Panel } from "../ui/Elements";
import { styles } from "../ui/theme";

export const SettingsScreen = () => (
  <View>
    <Text style={styles.overline}>VIDEO Y VISUALIZACIÓN · FASE M2</Text>
    <Text style={styles.title}>Ajustes</Text>
    <Text style={styles.subtitle}>Zoom inteligente que seguirá al cursor. Todavía no existe captura, reproducción de video ni configuración persistente.</Text>
    <Panel title="Streaming de pantalla" badge="PRÓXIMAMENTE">
      <Text style={styles.body}>Desactivado · pendiente de autorización y captura real del host Windows.</Text>
    </Panel>
    <Panel title="Zoom y seguimiento" badge="PRÓXIMAMENTE">
      <Text style={styles.body}>Zoom: desactivado. Seguir cursor: desactivado. Intensidad y suavizado se habilitarán con el renderizador M2.</Text>
    </Panel>
    <Panel title="Acceso remoto por internet" badge="FUERA DE M1">
      <Text style={styles.body}>No hay acceso celular, túneles, servidores públicos ni conexión a Internet. Fase M4 tras pruebas de LAN.</Text>
    </Panel>
  </View>
);
