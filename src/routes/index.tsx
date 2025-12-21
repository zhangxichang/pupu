import { useNavigate } from "@solidjs/router";
import { onMount } from "solid-js";

export default function Index() {
  const navigate = useNavigate();
  onMount(() => navigate("login"));
}
