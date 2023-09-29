const path = require('path');
const HtmlWebpackPlugin = require('html-webpack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin')
const TsconfigPathsPlugin = require('tsconfig-paths-webpack-plugin')

const dist = path.resolve(__dirname, 'dist')

module.exports = {
    experiments: {
        asyncWebAssembly: true,
        topLevelAwait: true
    },
    resolve: {
        plugins: [ new TsconfigPathsPlugin() ],
        extensions: ['.js', '.ts', '.tsx']
    },
    devtool: 'eval-cheap-module-source-map',
    devServer: {
        contentBase: dist,
        port: 9000,
        historyApiFallback: true
    },
    entry: {
        index: './js/index.tsx'
    },
    target: 'web',
    module: {
        rules: [
            {
                test: /\.wasm$/,
                type: 'asset/resource',
            },
            {
                test: /\.(js|ts|tsx)$/,
                exclude: /node_modules/,
                use: {
                    loader: 'babel-loader'
                }
            },
            {
                test: /\.(css|scss)$/,
                exclude: /node_modules/,
                use: ['style-loader', 'css-loader', 'sass-loader']
            },
            {
                test: /\.(png|ttf)/,
                type: 'asset/resource'
            }
        ]
    },
    output: {
        path: dist,
        filename: '[name].js'
    },
    plugins: [
        new HtmlWebpackPlugin({
            template: 'static/index.html'
        }),
        new CopyWebpackPlugin({
            patterns: [
                {from: '*.js', context: path.resolve(__dirname, 'static')}
            ]
        })
    ]
}