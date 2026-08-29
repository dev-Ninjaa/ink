import { Header } from "../components/header";
import { useUser } from "../../hooks/useUser";

export default function Page() {
  const user = useUser();
  return <Header name={user} />;
}
