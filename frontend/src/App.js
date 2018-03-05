import React, { Component } from 'react'
import { Chart } from 'react-google-charts'
import ReactQueryParams from 'react-query-params'
import logo from './logo.svg'
import './App.css'

const axios = require('axios')
const util = require('util')

const API_URL = 'http://localhost:1337/noise_levels?from=%d&to=%d'

async function getNoiseLevels(queryParams) {
  let from = queryParams.from === undefined
    ? 0
    : queryParams.from

  let to = queryParams.to === undefined 
    ? Math.floor(Date.now() / 1000)
    : queryParams.to

  let url = util.format(API_URL, from, to)

  let data = await axios.get(url)
    .then(res => {
      return res.data
    })

  data = mapData(data)
  return data
}

function findMinimum(data) {
  let min = 1
  data.forEach(function(i) {
    if (i[1] < min) min = i[1]
  })
  return min
}

function findMaximum(data) {
  let max = 0
  data.forEach(function(i) {
    if (i[1] > max) max = i[1]
  })
  return max
}

function mapData(data) {
  let min = findMinimum(data)
  let max = findMaximum(data)

  return data.map(function(i) {
    return [
      new Date(i[0] * 1000).toUTCString(),
      ((i[1] - min) * 100) / (max - min)
    ]
  })
}

class ExampleGoogleChart extends ReactQueryParams {
  constructor(props) {
    super(props)
    this.state = {
      options: {},
      data: {},
    }
    this.timerId = null;
  }

  async componentDidMount() {
    this.timerId = setInterval(async () => {
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
    }, 1000)
  }

  componentWillUnmount() {
    if (this.timerId !== null) {
      clearInterval(this.timerId)
    }
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

class App extends Component {
  render() {
    return (
      <div>
        <ExampleGoogleChart />
      </div>
    )
  }
}

export default App
