import { useNavigate, type RouteSectionProps } from "@solidjs/router";
import { onMount } from "solid-js";

export default function App(props: RouteSectionProps) {
  const navigate = useNavigate();
  onMount(() => navigate("login", { replace: true }));
  return props.children;
}
