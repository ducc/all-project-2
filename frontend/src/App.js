import React, { Component } from 'react'
import { Chart } from 'react-google-charts'
import ReactQueryParams from 'react-query-params'
import logo from './logo.svg'
import './App.css'

const axios = require('axios')
const util = require('util')

//const API_URL = 'http://localhost:1337/noise/decibels?from=%d&to=%d'
const API_URL = 'http://localhost:1337/battery/percentage?from=%d&to=%d'

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

  let values = data.slice().map(i => i[1]).sort((a, b) => a - b)
  let q1, q3
  if ((values.length / 4) % 1 === 0) {
    q1 = 1/2 * (values[(values.length / 4)] + values[(values.length / 4) + 1])
    q3 = 1/2 * (values[(values.length * (3 / 4))] + values[(values.length * (3 / 4)) + 1])
  } else {
    q1 = values[Math.floor(values.length / 4 + 1)];
    q3 = values[Math.ceil(values.length * (3 / 4) + 1)];
  }
  let iqr = q3 - q1;
  let maxValue = q3 + iqr * 1.5;
  let minValue = q1 - iqr * 1.5;

  return data
    .filter(function(i) {
      //return (i[1] >= minValue) && (i[1] <= maxValue)
      return true;
    })
    .map(function(i) {
      return [
        new Date(i[0] * 1000).toUTCString(),
        ((i[1] - min) * 100) / (max - min)
      ]
    })
}

class GoogleChart extends ReactQueryParams {
  constructor(props) {
    super(props)
    this.state = {
      options: {},
      data: {},
    }
    this.timerId = null;
  }

  async componentDidMount() {
    let fromTime = null
    if (this.queryParams.from === "now") {
      fromTime = Math.floor(Date.now() / 1000)
    } else {
      fromTime = this.queryParams.from
    }

    let oldData = null;

    this.timerId = setInterval(async () => {
      let queryParams = this.queryParams
      queryParams.from = fromTime
      let data = await getNoiseLevels(queryParams)

      /*if (oldData !== null) {
        data = oldData.concat(data)
      }

      oldData = data*/

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
        chartType='AreaChart'
        data={this.state.data}
        options={this.state.options}
        graph_id='AreaChart'
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
        <GoogleChart />
      </div>
    )
  }
}

export default App
