--
-- PostgreSQL database dump
--

-- Dumped from database version 12.4
-- Dumped by pg_dump version 12.4

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO postgres;

--
-- Name: choices; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.choices (
    id integer NOT NULL,
    details text NOT NULL,
    poll_id integer NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.choices OWNER TO postgres;

--
-- Name: choices_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.choices_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.choices_id_seq OWNER TO postgres;

--
-- Name: choices_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.choices_id_seq OWNED BY public.choices.id;


--
-- Name: polls; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.polls (
    id integer NOT NULL,
    uuid uuid NOT NULL,
    title text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.polls OWNER TO postgres;

--
-- Name: polls_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.polls_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.polls_id_seq OWNER TO postgres;

--
-- Name: polls_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.polls_id_seq OWNED BY public.polls.id;


--
-- Name: votes; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.votes (
    id integer NOT NULL,
    voter text NOT NULL,
    choice_id integer NOT NULL,
    poll_id integer NOT NULL,
    dots integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.votes OWNER TO postgres;

--
-- Name: votes_id_seq; Type: SEQUENCE; Schema: public; Owner: postgres
--

CREATE SEQUENCE public.votes_id_seq
    AS integer
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1;


ALTER TABLE public.votes_id_seq OWNER TO postgres;

--
-- Name: votes_id_seq; Type: SEQUENCE OWNED BY; Schema: public; Owner: postgres
--

ALTER SEQUENCE public.votes_id_seq OWNED BY public.votes.id;


--
-- Name: choices id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.choices ALTER COLUMN id SET DEFAULT nextval('public.choices_id_seq'::regclass);


--
-- Name: polls id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.polls ALTER COLUMN id SET DEFAULT nextval('public.polls_id_seq'::regclass);


--
-- Name: votes id; Type: DEFAULT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.votes ALTER COLUMN id SET DEFAULT nextval('public.votes_id_seq'::regclass);


--
-- Data for Name: _sqlx_migrations; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public._sqlx_migrations (version, description, installed_on, success, checksum, execution_time) FROM stdin;
20200919213146	polls	2020-10-11 19:41:45.749129+00	t	\\xe3793f7f17cb44156cc6756c32be5e847aa0949c8db360153b87646b4d9829438f733bc217a58a64c80706648f81fba5	160882735
20200919214013	choices	2020-10-11 19:41:46.049288+00	t	\\x91851e3571be9581e1573e8dbf54fc631ede7a178bdfa48276cf788f804f3ce815b2c54a5cae8c202f01f1389bb0d9c1	153960066
20200919214014	votes	2020-10-11 19:41:46.346082+00	t	\\x171a71eacad2f460e22fda3e11a1136649c71cba509b1f0b686f0dd445a77758891a40f2300d83a6f50f7e8feeff46cb	152215549
\.


--
-- Data for Name: choices; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.choices (id, details, poll_id, created_at) FROM stdin;
1	Jack Johnson	1	2020-10-11 20:17:46.937716+00
2	John Jackson	1	2020-10-11 20:17:46.937716+00
3	Bender	1	2020-10-11 20:17:46.937716+00
\.


--
-- Data for Name: polls; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.polls (id, uuid, title, created_at) FROM stdin;
1	e2b75a74-0dd9-4324-bc7e-4b1aa720f2f1	Who should be president	2020-10-11 20:17:46.937716+00
\.


--
-- Data for Name: votes; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.votes (id, voter, choice_id, poll_id, dots, created_at) FROM stdin;
1	Bender	3	1	3	2020-10-11 20:18:17.701145+00
2	Fry	3	1	1	2020-10-11 20:42:51.186259+00
3	Professor Farnsworth	1	1	1	2020-10-11 20:43:07.766275+00
4	Professor Farnsworth	2	1	1	2020-10-11 20:43:07.766275+00
5	Leela	1	1	2	2020-10-11 20:43:39.920032+00
\.


--
-- Name: choices_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.choices_id_seq', 3, true);


--
-- Name: polls_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.polls_id_seq', 5, true);


--
-- Name: votes_id_seq; Type: SEQUENCE SET; Schema: public; Owner: postgres
--

SELECT pg_catalog.setval('public.votes_id_seq', 5, true);


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: choices choices_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.choices
    ADD CONSTRAINT choices_pkey PRIMARY KEY (id);


--
-- Name: polls polls_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.polls
    ADD CONSTRAINT polls_pkey PRIMARY KEY (id);


--
-- Name: polls polls_uuid_key; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.polls
    ADD CONSTRAINT polls_uuid_key UNIQUE (uuid);


--
-- Name: votes votes_pkey; Type: CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.votes
    ADD CONSTRAINT votes_pkey PRIMARY KEY (id);


--
-- Name: uuid_idx; Type: INDEX; Schema: public; Owner: postgres
--

CREATE UNIQUE INDEX uuid_idx ON public.polls USING btree (uuid);


--
-- Name: votes fk_choice; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.votes
    ADD CONSTRAINT fk_choice FOREIGN KEY (choice_id) REFERENCES public.choices(id);


--
-- Name: choices fk_poll; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.choices
    ADD CONSTRAINT fk_poll FOREIGN KEY (poll_id) REFERENCES public.polls(id);


--
-- Name: votes fk_poll; Type: FK CONSTRAINT; Schema: public; Owner: postgres
--

ALTER TABLE ONLY public.votes
    ADD CONSTRAINT fk_poll FOREIGN KEY (poll_id) REFERENCES public.polls(id);


--
-- PostgreSQL database dump complete
--

