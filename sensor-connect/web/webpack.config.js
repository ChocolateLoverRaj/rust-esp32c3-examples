const path = require("path");
const WasmPackPlugin = require("@wasm-tool/wasm-pack-plugin");
const HtmlWebpackPlugin = require('html-webpack-plugin')
const WorkboxPlugin = require('workbox-webpack-plugin');

const dist = path.resolve(__dirname, "dist");

module.exports = (env) => ({
  mode: "development",
  experiments: {
    asyncWebAssembly: true,
  },
  entry: {
    index: [
      "./pkg/index.js",
      ...env.WEBPACK_BUILD
        ? ["./js/register-sw.js"]
        : []
    ],
  },
  output: {
    path: dist,
    filename: "[name].js",
  },
  devServer: {
    static: dist,
  },
  plugins: [
    new HtmlWebpackPlugin({
      template: "index.html"
    }),
    new WasmPackPlugin({
      crateDirectory: __dirname,
      env: {
        RUSTFLAGS: "--cfg=web_sys_unstable_apis"
      }
    }),
    ...env.WEBPACK_BUILD
      ? [
        new WorkboxPlugin.GenerateSW({
          // these options encourage the ServiceWorkers to get in there fast
          // and not allow any straggling "old" SWs to hang around
          clientsClaim: true,
          skipWaiting: true,
          swDest: 'service-worker.js'
        })
      ]
      : [],
  ]
});
