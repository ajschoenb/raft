import React from 'react';
import Button from 'react-bootstrap/Button';
import Container from 'react-bootstrap/Container';
import Table from 'react-bootstrap/Table';
import './App.css';

class LogView extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            logs: [[],[],[],[],[]],
            statuses: ['follower','follower','follower','follower','follower']
        };
    }

    componentDidMount() {
        this.timer = setInterval(
            () => this.tick(),
            250
        );
    }

    componentWillUnmount() {
        clearInterval(this.timer);
    }

    async tick() {
        let res = await fetch('/state');
        let data = await res.json();
        this.setState({logs: data.logs, statuses: data.statuses});
    }

    renderData(idx) {
        let logs = this.state.logs;
        let statuses = this.state.statuses;
        let logsLen = logs[idx].length;
        let firstIdx = Math.max(logsLen - 20, 0);
        return (
            <div>
                <Table bordered variant="dark">
                    <tr>
                        <th>Server {idx}</th>
                        <th>Term</th>
                        {logs[idx].slice(firstIdx).map(e => {
                            return (<td className="tdentry">{e['term']}</td>);
                        })}
                    </tr>
                    <tr>
                        {statuses[idx] === 'Crashed' &&
                            <th style={{'color': 'Tomato'}}>Crashed</th>
                        }
                        {statuses[idx] === 'Follower' &&
                            <th style={{'color': 'LightGreen'}}>Follower</th>
                        }
                        {statuses[idx] === 'Leader' &&
                            <th style={{'color': 'Plum'}}>Leader</th>
                        }
                        <th>Opid</th>
                        {logs[idx].slice(firstIdx).map(e => {
                            return (<td className="tdentry">{e['opid']}</td>);
                        })}
                    </tr>
                </Table>
            </div>
        );
    }

    render() {
        return (
            <div>
                {this.renderData(0)}
                {this.renderData(1)}
                {this.renderData(2)}
                {this.renderData(3)}
                {this.renderData(4)}
            </div>
        );
    }
}

class RequestButton extends React.Component {
    request() {
        fetch('/request', {method:'POST'});
    }

    render() {
        return (
            <Button size="lg" onClick={this.request}>Make a Request</Button>
        );
    }
}

function App() {
    return (
        <div style={{margin: '25px'}}>
            <Container fluid>
                <center>
                    <LogView />
                    <RequestButton />
                </center>
            </Container>
        </div>
    );
}

export default App;
