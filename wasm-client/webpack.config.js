const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const webpack = require('webpack');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');

module.exports = {
    entry: './index.js',
    output: {
        path: path.resolve(__dirname, 'dist'),
        filename: 'index.js',
    },
    plugins: [
        new HtmlWebpackPlugin(),
        new WasmPackPlugin({
            crateDirectory: path.resolve(__dirname, ".")
        }),
        // Edge which doesn't ship `TextEncoder` or `TextDecoder` at this time.
        new webpack.ProvidePlugin({
            TextEncoder: ['text-encoding', 'TextEncoder'],
            TextDecoder: ['text-encoding', 'TextDecoder'],
        })
    ],
    mode: 'development'
}