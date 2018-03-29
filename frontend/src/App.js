import React, { Component } from 'react'
import { Chart } from 'react-google-charts'
import ReactQueryParams from 'react-query-params'
import './App.css'
import Config from './config.js'

const axios = require('axios')
const util = require('util')

const API_URL = Config.API_URL + '%s?from=%d&to=%d'

async function getReadings(graphId, queryParams) {
  let from = queryParams.from === undefined
    ? 0
    : queryParams.from

  let to = queryParams.to === undefined 
    ? Math.floor(Date.now() / 1000)
    : queryParams.to

  let url = util.format(API_URL, graphId, from, to)

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

  return data
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
      data: {}
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

    this.timerId = setInterval(async () => {
      let queryParams = this.queryParams
      queryParams.from = fromTime
      let data = await getReadings(this.props.graphId, queryParams)

      this.setState({
        options: {
          title: 'Time vs. ' + this.props.name + ' comparison',
          hAxis: { title: 'Time', minValue: 0 },
          vAxis: { title: this.props.name, minValue: 0 },
          legend: 'none',
        },
        data: [['Time', this.props.name]].concat(data),
      })
    }, 1000)
  }

  componentWillUnmount() {
    if (this.timerId !== null) {
      clearInterval(this.timerId)
    }
  }

  render() {
    if (Object.keys(this.state.data).length === 0 && this.state.data.constructor === Object) {
      return null
    }

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
  constructor(props) {
    super(props)
    this.state = {
      value: 0,
    }
    this.handleChange = this.handleChange.bind(this);
  }
  
  handleChange(event) {
    this.setState({
      value: event.target.value,
    })
  }

  getOption(value) {
    if (value == 0) {
      return ['noise/decibels', 'Noise Levels']
    } else if (value == 1) {
      return ['battery/percentage', 'Battery Percentage']
    } else {
      console.log("Oh no!!! unknown option!!!!")
    }
  }

  render() {
    let graphId = this.getOption(this.state.value)[0];
    let graphName = this.getOption(this.state.value)[0];

    return (
      <div>
        <form>
          <select onChange={this.handleChange}>
            <option defaultValue value={0}>Noise Levels</option>
            <option value={1}>Battery Percentage</option>
          </select>
        </form>
        <GoogleChart graphId={graphId} name={graphName} />
      </div>
    )
  }
}

export default App
