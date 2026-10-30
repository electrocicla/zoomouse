import { Text, TextInput, View } from "react-native";
import { Action, Panel } from "../ui/Elements";
import { palette, styles } from "../ui/theme";

export const ControlsScreen = ({ onApproval, onSettings }: {
  onApproval: () => void;
  onSettings: () => void;
}) => (
  <View>
    <Text style={styles.overline}>PANEL DE CONTROL · BLOQUEADO</Text>
    <Text style={styles.title}>Mouse y teclado</Text>
    <Text style={styles.subtitle}>Estas superficies son un diseño de pantalla. No se envían pulsaciones, movimiento ni texto a Windows.</Text>
    <Panel title="Touchpad" badge="SIN SESIÓN APROBADA">
      <View accessible accessibilityLabel="Touchpad deshabilitado hasta tener una sesión autorizada"
        style={{ height: 182, borderRadius: 14, backgroundColor: palette.background, borderWidth: 1, borderColor: palette.border, justifyContent: "center", alignItems: "center", marginTop: 7 }}>
        <Text style={[styles.body, { textAlign: "center" }]}>Desliza para mover el cursor{"\n"}(próximamente)</Text>
      </View>
      <Action label="Clic izquierdo (deshabilitado)" disabled onPress={() => {}} secondary />
      <Action label="Clic derecho (deshabilitado)" disabled onPress={() => {}} secondary />
    </Panel>
    <Panel title="Teclado remoto" badge="NO DISPONIBLE">
      <TextInput accessibilityLabel="Entrada de teclado remoto no disponible" style={styles.input}
        placeholder="El texto no se enviará" placeholderTextColor={palette.disabled} editable={false} />
    </Panel>
    <Action label="Consultar aprobación" onPress={onApproval} secondary />
    <Action label="Opciones de zoom" onPress={onSettings} secondary />
  </View>
);
