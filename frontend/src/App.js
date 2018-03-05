import React, { Component } from 'react'
import { Chart } from 'react-google-charts'
import { LineChart, Line } from 'recharts'
import ReactQueryParams from 'react-query-params'
import logo from './logo.svg'
import './App.css'

const axios = require('axios')
const util = require('util')

const API_URL = 'http://localhost:1337/noise_levels?from=%d&to=%d'

async function getNoiseLevels(queryParams) {
  let from = queryParams.from === undefined
    ? 0 : queryParams.from

  let to = queryParams.to === undefined 
    ? Math.floor(Date.now() / 1000)
    : queryParams.to

  console.log('from, to: ' + from + ', ' + to)

  let url = util.format(API_URL, from, to)

  let data = await axios.get(url)
    .then(res => {
      return res.data
    })

  console.log('data: ' + JSON.stringify(data))
  return data
}

class ExampleGoogleChart extends ReactQueryParams {
  constructor(props) {
    super(props)
    this.state = {
      options: {},
      data: {},
    }
  }

  async componentDidMount() {
      let data = await getNoiseLevels(this.queryParams)

      this.setState({
        options: {
          title: 'Time vs. Noise level comparison',
          hAxis: { title: 'Time', minValue: 0 },
          vAxis: { title: 'Noise level', minValue: 0 },
          legend: 'none',
        },
        data: [['Time', 'Noise level']].concat(data),
      })

      console.log("data2: " + JSON.stringify(this.state.data));
  }

  render() {
    return (
      <Chart
        chartType='ScatterChart'
        data={this.state.data}
        options={this.state.options}
        graph_id='ScatterChart'
        width='100%'
        height='400px'
        legend_toggle
      />
    )
  }
}

class ExampleRechartsChart extends Component {
  constructor(props) {
    super(props)
  }

  async componentDidMount() {
    // get data, set state
  }

  render() {
    return (
      <p>todo</p>
    )
  }
}

class App extends Component {
  render() {
    return (
      <div>
        <ExampleGoogleChart />
        {/*<ExampleRechartsChart />*/}
      </div>
    )
  }
}

export default App
