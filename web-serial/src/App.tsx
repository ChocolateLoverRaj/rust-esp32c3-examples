import { FC, useLayoutEffect, useRef, useState } from 'react';
import { Helmet } from 'react-helmet'
import './style.css';

interface PortData {
  port: SerialPort,
  output: Uint8Array
}

export const App: FC<{ name: string }> = ({ name }) => {
  const [portData, setPortData] = useState<PortData>()
  const [input, setInput] = useState('')
  const logRef = useRef<HTMLPreElement>(null)

  useLayoutEffect(() => {
    // Scroll to bottom
    logRef.current.scrollTo(0, Number.MAX_SAFE_INTEGER)

  }, [portData?.output])

  return (
    <>
      <Helmet>
        <title>Web Serial Viewer</title>
      </Helmet>
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
        <button
          disabled={portData !== undefined}
          onClick={async () => {
            const port = await navigator.serial.requestPort()
            await port.open({ baudRate: 460800 })
            setPortData({
              port,
              output: new Uint8Array()
            })
            const reader = port.readable.getReader()
            while (true) {
              const { value, done } = await reader.read();
              if (done) {
                // |reader| has been canceled.
                break;
              }
              setPortData(data => ({
                ...data,
                output: new Uint8Array([...data.output, ...value])
              }))
            }
          }}
        >
          {portData === undefined ? 'Connect' : 'Connected'}
        </button><br />

        <form
          onSubmit={async e => {
            e.preventDefault()
            console.log(input)
            setInput('')

            const writer = portData.port.writable.getWriter()
            await writer.write(new TextEncoder().encode(`${input}\n`))
            writer.releaseLock()
          }}
        >
          <label>
            Send line of input <br />
            <input style={{ fontFamily: 'monospace' }} value={input} onChange={({ target: { value } }) => setInput(value)} />
            <button type='submit' disabled={portData === undefined}>Send</button>
          </label>
        </form>

        Output:<br />
        <pre ref={logRef} style={{ flexGrow: 1, overflow: 'auto' }}><code>{portData !== undefined ? new TextDecoder().decode(portData.output) : <i>No output yet</i>}</code></pre>
      </div>
    </>
  );
};
