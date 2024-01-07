import { FC, useState } from 'react';
import { Helmet } from 'react-helmet'
import './style.css';

export const App: FC<{ name: string }> = ({ name }) => {
  const [output, setOutput] = useState<Uint8Array>()

  return (
    <>
      <Helmet>
        <title>Web Serial Viewer</title>
      </Helmet>
      <button onClick={async () => {
        const port = await navigator.serial.requestPort()
        await port.open({ baudRate: 460800 })
        const reader = port.readable.getReader()
        while (true) {
          const { value, done } = await reader.read();
          if (done) {
            // |reader| has been canceled.
            break;
          }
          setOutput(output => output === undefined ? value : new Uint8Array([...output, ...value]))
          // Hacks
          document.body.scrollTo(0, Number.MAX_SAFE_INTEGER)
        }
      }}>Connect to ESP32-C3 (or something else, doesn't really have to be an ESP32-C3)</button><br />
      Output:<br />
      <pre><code>{output !== undefined ? new TextDecoder().decode(output) : <i>No output yet</i>}</code></pre>
    </>
  );
};
