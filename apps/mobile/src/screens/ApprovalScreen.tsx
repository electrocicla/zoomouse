import { Text, View } from "react-native";
import { Action, Panel } from "../ui/Elements";
import { styles } from "../ui/theme";

export const ApprovalScreen = ({ onBack }: { onBack: () => void }) => (
  <View>
    <Text style={styles.overline}>PERMISO EXPLÍCITO</Text>
    <Text style={styles.title}>El host decide.</Text>
    <Text style={styles.subtitle}>El usuario en Windows deberá aprobar la identidad del teléfono antes de habilitar teclado, mouse o video.</Text>
    <Panel title="Estado real" badge="NINGUNA SOLICITUD ACTIVA">
      <Text style={styles.body}>No existe sesión de emparejamiento. Al implementar M1-D se mostrará la solicitud real y su resultado firmado.</Text>
    </Panel>
    <Panel title="Protecciones">
      <Text style={styles.body}>La autorización no se concede desde Android. Revocar, desconectar o caducar la sesión bloqueará la entrada y liberará teclas retenidas.</Text>
    </Panel>
    <Action label="Volver a dispositivos" onPress={onBack} secondary />
  </View>
);
