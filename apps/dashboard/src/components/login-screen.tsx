import { Button } from "@kira/ui/components/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "@kira/ui/components/card";
import { useState, type FormEvent } from "react";

import { useLoginMutation } from "@/data/auth/mutation";

export function LoginScreen() {
  const [pin, setPin] = useState("");
  const login = useLoginMutation();
  const error = login.error?.message;

  function onSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    login.mutate(pin);
  }

  return (
    <main className="flex min-h-svh flex-1 items-center justify-center bg-background p-6 text-foreground">
      <Card className="w-full max-w-sm">
        <form onSubmit={onSubmit}>
          <CardHeader>
            <CardTitle>Kira Dashboard</CardTitle>
            <CardDescription>
              Enter the PIN printed by the Kira server.
            </CardDescription>
          </CardHeader>
          <CardContent className="flex flex-col gap-2">
            <label htmlFor="dashboard-pin" className="text-sm font-medium">
              PIN
            </label>
            <input
              id="dashboard-pin"
              type="password"
              inputMode="numeric"
              autoComplete="current-password"
              autoFocus
              value={pin}
              onChange={(event) => setPin(event.target.value)}
              className="h-9 rounded-md border border-input bg-transparent px-3 text-sm shadow-xs outline-none focus-visible:ring-2 focus-visible:ring-ring"
            />
            {error ? (
              <p role="alert" className="text-sm text-destructive">
                {error}
              </p>
            ) : null}
          </CardContent>
          <CardFooter>
            <Button
              type="submit"
              className="w-full"
              disabled={login.isPending || pin.trim().length === 0}
            >
              {login.isPending ? "Signing in…" : "Sign in"}
            </Button>
          </CardFooter>
        </form>
      </Card>
    </main>
  );
}
