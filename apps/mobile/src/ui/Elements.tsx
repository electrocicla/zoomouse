import type { ReactNode } from "react";
import { Pressable, Text, View } from "react-native";
import { styles } from "./theme";

export const Panel = ({ title, children, badge }: { title: string; children: ReactNode; badge?: string }) => (
  <View style={styles.card}>
    {badge ? <Text style={styles.badge}>{badge}</Text> : null}
    <Text style={styles.cardTitle}>{title}</Text>
    {children}
  </View>
);

export const Action = ({ label, onPress, secondary = false, disabled = false }: {
  label: string;
  onPress: () => void;
  secondary?: boolean;
  disabled?: boolean;
}) => (
  <Pressable
    accessibilityRole="button"
    accessibilityState={{ disabled }}
    disabled={disabled}
    onPress={onPress}
    style={[styles.button, secondary && styles.secondaryButton, disabled && styles.disabledButton]}
  >
    <Text style={[styles.buttonLabel, secondary && styles.secondaryLabel]}>{label}</Text>
  </Pressable>
);
