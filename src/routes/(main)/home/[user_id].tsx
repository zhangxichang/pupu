import { useParams } from "@solidjs/router";

export default function Home() {
  const params = useParams<{ user_id: string }>();
  return <div>主页，用户ID:{params.user_id}</div>;
}
