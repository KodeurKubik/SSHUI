// why am i doing this in TS? idk cause i used to code in ts
// BUT WHY IS FS NOT RECOGNIZED IN VSCODE??? I DONT KNOOOOOWWW
// oh it was missing a ts config :skull:
import { createWriteStream, readdirSync, readFileSync } from "fs";

const input_name = prompt("Input name", "badapple")!;

const output = createWriteStream(`./${input_name}.ascii`, {
  encoding: "utf-8",
  flush: true,
});

let frames = readdirSync(`./${input_name}-frames`)
  .sort((a, b) => a.localeCompare(b))
  .filter((_, i) => i % 2 == 0);

for (let frame of frames) {
  console.log(`frame ${frame}`);
  const content = readFileSync(`./${input_name}-frames/${frame}`, "utf-8");
  let inner = content.split("\n");

  output.write("\n" + inner.map((l) => l).join(""));
}
