import { type FC, useLayoutEffect, useRef, useState } from 'react'
import { Helmet } from 'react-helmet'
import { parse } from 'ansicolor'
import unwrap from 'throw-empty'
import './style.css'

interface PortData {
  port: SerialPort
  output: Uint8Array
}

export const App: FC<{ name: string }> = ({ name }) => {
  const [portData, setPortData] = useState<PortData>()
  const [input, setInput] = useState('')
  const logRef = useRef<HTMLPreElement>(null)

  useLayoutEffect(() => {
    // Scroll to bottom
    logRef.current?.scrollTo(0, Number.MAX_SAFE_INTEGER)
  }, [portData?.output])

  return (
    <>
      <Helmet>
        <title>Web Serial Viewer</title>
      </Helmet>
      <div style={{ display: 'flex', flexDirection: 'column', height: '100%' }}>
        <button
          disabled={portData !== undefined}
          onClick={() => {
            (async () => {
              const port = await navigator.serial.requestPort()
              await port.open({ baudRate: 460800 })
              setPortData({
                port,
                output: new Uint8Array()
              })
              const reader = unwrap(port.readable).getReader()
              while (true) {
                const { value, done } = await reader.read()
                if (done) {
                  // |reader| has been canceled.
                  break
                }
                setPortData(data => {
                  if (data === undefined) {
                    throw new Error()
                  }
                  return {
                    ...data,
                    output: new Uint8Array([...data.output, ...value])
                  }
                })
              }
            })().catch(e => {
              console.error(e)
            })
          }}
        >
          {portData === undefined ? 'Connect' : 'Connected'}
        </button><br />

        <form
          onSubmit={e => {
            (async () => {
              e.preventDefault()
              console.log(input)
              setInput('')

              const writer = unwrap(unwrap(portData).port.writable).getWriter()
              await writer.write(new TextEncoder().encode(`${input}\n`))
              writer.releaseLock()
            })().catch(e => {
              console.error(e)
            })
          }}
        >
          <label>
            Send line of input <br />
            <input style={{ fontFamily: 'monospace' }} value={input} onChange={({ target: { value } }) => { setInput(value) }} />
            <button type='submit' disabled={portData === undefined}>Send</button>
          </label>
        </form>

        Output:<br />
        <pre ref={logRef} style={{ flexGrow: 1, overflow: 'auto' }}>
          <code>
            {portData !== undefined
              ? parse(new TextDecoder().decode(portData.output)).spans.map(({ color, text }, index) =>
                <span style={{ color: color?.name }} key={index}>{text}</span>
              )
              : <i>No output yet</i>}
          </code>
        </pre>
      </div>
    </>
  )
}
